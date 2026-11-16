use super::{wide, Result};
use crate::config::Settings;
use windows::core::{ComInterface, IUnknown, Interface, HRESULT, PCWSTR, PWSTR};
use windows::Win32::Foundation::{WAIT_OBJECT_0, WAIT_TIMEOUT};
use windows::Win32::Media::Speech::*;
use windows::Win32::System::Com::{CoCreateInstance, CoTaskMemFree, CLSCTX_ALL};
use windows::Win32::System::Threading::WaitForSingleObject;

fn tokens(language: u16) -> Result<IEnumSpObjectTokens> {
    unsafe {
        let category: ISpObjectTokenCategory =
            CoCreateInstance(&SpObjectTokenCategory, None, CLSCTX_ALL)
                .map_err(|e| e.to_string())?;
        category
            .SetId(SPCAT_RECOGNIZERS, false)
            .map_err(|e| e.to_string())?;
        let attributes = wide(&if language == 0 {
            String::new()
        } else {
            format!("Language={language:X}")
        });
        category
            .EnumTokens(PCWSTR(attributes.as_ptr()), PCWSTR::null())
            .map_err(|e| e.to_string())
    }
}

pub fn diagnostic() -> Result<String> {
    unsafe {
        let tokens = tokens(0)?;
        let mut count = 0;
        tokens.GetCount(&mut count).map_err(|e| e.to_string())?;
        let mut report = format!("Installed SAPI recognizers: {count}");
        for index in 0..count {
            let token = tokens.Item(index).map_err(|e| e.to_string())?;
            let name = token
                .GetStringValue(PCWSTR::null())
                .map_err(|e| e.to_string())?;
            let text = name.to_string().unwrap_or_default();
            CoTaskMemFree(Some(name.0.cast()));
            report.push_str(&format!("\n- {text}"));
        }
        Ok(report)
    }
}

pub struct Listener {
    recognizer: ISpRecognizer,
    context: ISpRecoContext,
    _grammar: ISpRecoGrammar,
    debug: bool,
}

#[derive(Debug)]
pub enum SpeechEvent {
    Recognized {
        index: usize,
        text: String,
        confidence: i8,
    },
    Trace(String),
    AudioLevel(u8),
}

unsafe fn take_string(text: PWSTR) -> String {
    let value = text.to_string().unwrap_or_default();
    CoTaskMemFree(Some(text.0.cast()));
    value
}

fn default_audio_input() -> Result<ISpObjectToken> {
    unsafe {
        let category: ISpObjectTokenCategory =
            CoCreateInstance(&SpObjectTokenCategory, None, CLSCTX_ALL)
                .map_err(|e| e.to_string())?;
        category
            .SetId(SPCAT_AUDIOIN, false)
            .map_err(|e| e.to_string())?;
        let id = category
            .GetDefaultTokenId()
            .map_err(|e| format!("No default SAPI microphone: {e}"))?;
        let token: ISpObjectToken =
            CoCreateInstance(&SpObjectToken, None, CLSCTX_ALL).map_err(|e| e.to_string())?;
        let result = token.SetId(PCWSTR::null(), PCWSTR(id.0), false);
        CoTaskMemFree(Some(id.0.cast()));
        result.map_err(|e| e.to_string())?;
        Ok(token)
    }
}

fn event_interest(debug: bool) -> u64 {
    let mut interest = (1u64 << 30) | (1 << 33);
    for event in [
        SPEI_RECOGNITION,
        SPEI_FALSE_RECOGNITION,
        SPEI_START_SR_STREAM,
        SPEI_END_SR_STREAM,
    ] {
        interest |= 1 << event.0;
    }
    if debug {
        for event in [
            SPEI_SOUND_START,
            SPEI_SOUND_END,
            SPEI_PHRASE_START,
            SPEI_HYPOTHESIS,
            SPEI_SR_AUDIO_LEVEL,
            SPEI_INTERFERENCE,
        ] {
            interest |= 1 << event.0;
        }
    }
    interest
}

impl Listener {
    pub fn new(settings: &Settings) -> Result<Self> {
        unsafe {
            let recognizer: ISpRecognizer = CoCreateInstance(&SpInprocRecognizer, None, CLSCTX_ALL)
                .map_err(|e| e.to_string())?;
            let tokens = tokens(settings.language)?;
            let mut count = 0;
            tokens.GetCount(&mut count).map_err(|e| e.to_string())?;
            if count == 0 {
                return Err("No SAPI recognizer is installed for this language. Install its Windows speech recognition pack or select another language.".into());
            }
            let token = tokens.Item(0).map_err(|e| e.to_string())?;
            recognizer
                .SetRecognizer(&token)
                .map_err(|e| e.to_string())?;
            // In-process recognizers require a real input token. NULL only
            // refreshes the default device for a shared recognizer.
            let input = default_audio_input()?;
            recognizer
                .SetInput(&input, true)
                .map_err(|e| format!("SAPI microphone: {e}"))?;
            recognizer
                .GetInputObjectToken()
                .map_err(|e| format!("SAPI audio input is not attached: {e}"))?;
            let context = recognizer.CreateRecoContext().map_err(|e| e.to_string())?;
            context.SetNotifyWin32Event().map_err(|e| e.to_string())?;
            let interest = event_interest(false);
            context
                .SetInterest(interest, interest)
                .map_err(|e| e.to_string())?;
            let grammar = context.CreateGrammar(1).map_err(|e| e.to_string())?;
            grammar
                .ResetGrammar(settings.language)
                .map_err(|e| e.to_string())?;
            for (index, trigger) in settings.triggers.iter().enumerate() {
                let mut state = std::ptr::null_mut();
                grammar
                    .GetRule(
                        PCWSTR::null(),
                        index as u32 + 1,
                        (SPRAF_TopLevel.0 | SPRAF_Dynamic.0) as u32,
                        true,
                        &mut state,
                    )
                    .map_err(|e| e.to_string())?;
                for phrase in pronunciations(&trigger.phrase) {
                    grammar
                        .AddWordTransition(
                            state,
                            std::ptr::null_mut(),
                            PCWSTR(wide(&phrase).as_ptr()),
                            windows::core::w!(" "),
                            SPWT_LEXICAL,
                            1.0,
                            std::ptr::null(),
                        )
                        .map_err(|e| format!("Trigger phrase '{}': {e}", trigger.phrase))?;
                }
            }
            grammar.Commit(0).map_err(|e| e.to_string())?;
            // SetRuleState(NULL) only affects rules created with SPRAF_Active.
            // Activate each configured rule explicitly instead of relying on it.
            for index in 0..settings.triggers.len() {
                grammar
                    .SetRuleIdState(index as u32 + 1, SPRS_ACTIVE)
                    .map_err(|e| format!("Could not activate trigger rule {}: {e}", index + 1))?;
            }
            let mut status = SPRECOGNIZERSTATUS::default();
            recognizer
                .GetStatus(&mut status)
                .map_err(|e| e.to_string())?;
            if status.ulNumActive == 0 {
                return Err(
                    "SAPI has no active recognition grammar. Listening was not started.".into(),
                );
            }
            recognizer
                .SetRecoState(SPRST_ACTIVE_ALWAYS)
                .map_err(|e| e.to_string())?;
            Ok(Self {
                recognizer,
                context,
                _grammar: grammar,
                debug: false,
            })
        }
    }

    pub fn set_debug(&mut self, enabled: bool) -> Result<()> {
        if enabled != self.debug {
            let interest = event_interest(enabled);
            unsafe { self.context.SetInterest(interest, interest) }.map_err(|e| e.to_string())?;
            self.debug = enabled;
        }
        Ok(())
    }

    pub fn input_name(&self) -> Result<String> {
        unsafe {
            let token = self
                .recognizer
                .GetInputObjectToken()
                .map_err(|e| e.to_string())?;
            Ok(take_string(
                token
                    .GetStringValue(PCWSTR::null())
                    .map_err(|e| e.to_string())?,
            ))
        }
    }

    pub fn poll(&self) -> Result<Vec<SpeechEvent>> {
        unsafe {
            let wait = WaitForSingleObject(self.context.GetNotifyEventHandle(), 100);
            if wait == WAIT_TIMEOUT {
                return Ok(Vec::new());
            }
            if wait != WAIT_OBJECT_0 {
                return Err("Could not wait for SAPI audio events.".into());
            }
            let mut output = Vec::new();
            loop {
                let mut event = SPEVENT::default();
                let mut fetched = 0;
                self.context
                    .GetEvents(1, &mut event, &mut fetched)
                    .map_err(|e| e.to_string())?;
                if fetched == 0 {
                    break;
                }
                let id = event._bitfield & 0xffff;
                // GetEvents transfers one COM reference; this owner releases it
                // for recognition, hypothesis and rejection events alike.
                let object = (event.lParam.0 != 0
                    && (event._bitfield >> 16) == SPET_LPARAM_IS_OBJECT.0)
                    .then(|| IUnknown::from_raw(event.lParam.0 as *mut _));
                if [
                    SPEI_RECOGNITION.0,
                    SPEI_HYPOTHESIS.0,
                    SPEI_FALSE_RECOGNITION.0,
                ]
                .contains(&id)
                {
                    if let Some(object) = object {
                        let result: ISpRecoResult = object.cast().map_err(|e| e.to_string())?;
                        let phrase = result.GetPhrase().map_err(|e| e.to_string())?;
                        let rule = (*phrase).Base.Rule.ulId;
                        let confidence = (*phrase).Base.Rule.Confidence;
                        CoTaskMemFree(Some(phrase.cast()));
                        let mut text = PWSTR::null();
                        result
                            .GetText(0, u32::MAX, true, &mut text, None)
                            .map_err(|e| e.to_string())?;
                        let text = take_string(text);
                        if id == SPEI_RECOGNITION.0 && rule > 0 && confidence >= 0 {
                            output.push(SpeechEvent::Recognized {
                                index: rule as usize - 1,
                                text,
                                confidence,
                            });
                        } else if id == SPEI_HYPOTHESIS.0 {
                            output.push(SpeechEvent::Trace(format!("Partial match: {text}")));
                        } else {
                            output.push(SpeechEvent::Trace(format!("Rejected: {text} (confidence {confidence}; a trigger needs 0 or 1).")));
                        }
                    } else {
                        output.push(SpeechEvent::Trace(
                            "Speech detected, but no configured phrase matched.".into(),
                        ));
                    }
                } else if id == SPEI_SR_AUDIO_LEVEL.0 {
                    output.push(SpeechEvent::AudioLevel(event.wParam.0.min(100) as u8));
                } else if id == SPEI_START_SR_STREAM.0 {
                    output.push(SpeechEvent::Trace(
                        "Microphone stream started; audio is reaching SAPI.".into(),
                    ));
                } else if id == SPEI_END_SR_STREAM.0 {
                    let result = HRESULT(event.lParam.0 as i32);
                    return Err(format!("SAPI microphone stream ended ({result:?}). Check the input device and restart listening."));
                } else if id == SPEI_SOUND_START.0 {
                    output.push(SpeechEvent::Trace("Sound detected.".into()));
                } else if id == SPEI_SOUND_END.0 {
                    output.push(SpeechEvent::Trace("Sound ended.".into()));
                } else if id == SPEI_PHRASE_START.0 {
                    output.push(SpeechEvent::Trace(
                        "Speech detected; matching configured phrases…".into(),
                    ));
                } else if id == SPEI_INTERFERENCE.0 {
                    output.push(SpeechEvent::Trace(format!(
                        "Audio interference reported by SAPI (code {}).",
                        event.lParam.0
                    )));
                }
            }
            Ok(output)
        }
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        unsafe {
            let _ = self.recognizer.SetRecoState(SPRST_INACTIVE_WITH_PURGE);
        }
    }
}

fn pronunciations(phrase: &str) -> Vec<String> {
    let phrase = crate::config::normalize(phrase);
    let mut phrases = vec![phrase.clone()];
    if phrase.contains("chatgpt") {
        phrases.push(phrase.replace("chatgpt", "chat gé pé té"));
        phrases.push(phrase.replace("chatgpt", "chat g p t"));
    } else if phrase.split_whitespace().any(|word| word == "gpt") {
        for spoken in ["gé pé té", "g p t"] {
            phrases.push(
                phrase
                    .split_whitespace()
                    .map(|word| if word == "gpt" { spoken } else { word })
                    .collect::<Vec<_>>()
                    .join(" "),
            );
        }
    }
    phrases
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_standalone_gpt_acronym_without_rewriting_other_words() {
        assert_eq!(
            pronunciations("Ok GPT"),
            vec!["ok gpt", "ok gé pé té", "ok g p t"]
        );
        assert_eq!(pronunciations("ok gptool"), vec!["ok gptool"]);
    }

    #[test]
    #[ignore = "Requires the installed French SAPI engine and the real default microphone"]
    fn listener_attaches_the_default_microphone() {
        let _com = crate::platform::Com::new().unwrap();
        let listener = Listener::new(&Settings::default()).unwrap();
        unsafe {
            listener
                .recognizer
                .GetInputObjectToken()
                .expect("An in-process recognizer must have an actual audio input token");
        }
    }

    #[test]
    #[ignore = "Requires the installed French SAPI engine and the real default microphone"]
    fn listener_receives_live_audio_events() {
        let _com = crate::platform::Com::new().unwrap();
        let mut listener = Listener::new(&Settings::default()).unwrap();
        listener.set_debug(true).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while std::time::Instant::now() < deadline {
            if listener
                .poll()
                .unwrap()
                .iter()
                .any(|event| matches!(event, SpeechEvent::AudioLevel(_)))
            {
                return;
            }
        }
        panic!("The microphone must produce live SAPI level events, even in silence");
    }

    #[test]
    #[ignore = "Requires the installed French SAPI engine; emulates text without running any action"]
    fn configured_phrase_is_dispatched_as_a_trigger() {
        use std::mem::ManuallyDrop;
        use windows::core::BSTR;
        use windows::Win32::System::Com::{VARIANT, VT_BSTR};
        let _com = crate::platform::Com::new().unwrap();
        let settings = Settings::default();
        let listener = Listener::new(&settings).unwrap();
        unsafe {
            let automation: ISpeechRecognizer = listener.recognizer.cast().unwrap();
            let mut words = VARIANT::default();
            let inner = &mut *words.Anonymous.Anonymous;
            inner.vt = VT_BSTR;
            inner.Anonymous.bstrVal =
                ManuallyDrop::new(BSTR::from(settings.triggers[0].phrase.as_str()));
            let result = automation.EmulateRecognition(
                words.clone(),
                &VARIANT::default(),
                settings.language as i32,
            );
            windows::Win32::System::Ole::VariantClear(&mut words).unwrap();
            result.unwrap();
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        let mut observed = Vec::new();
        while std::time::Instant::now() < deadline {
            let events = listener.poll().unwrap();
            if events
                .iter()
                .any(|event| matches!(event, SpeechEvent::Recognized { index: 0, .. }))
            {
                return;
            }
            observed.extend(events);
        }
        panic!("The configured phrase must reach the trigger dispatcher: {observed:?}");
    }
}
