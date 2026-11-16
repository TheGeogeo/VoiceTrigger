use super::{wide, Result};
use crate::config::Settings;
use windows::core::{ComInterface, IUnknown, Interface, PCWSTR};
use windows::Win32::Foundation::WAIT_OBJECT_0;
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
        let mut report = format!("Moteurs vocaux SAPI : {count}");
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
                return Err("Aucun moteur SAPI pour cette langue. Installez la reconnaissance vocale française dans Windows ou choisissez un autre moteur.".into());
            }
            let token = tokens.Item(0).map_err(|e| e.to_string())?;
            recognizer
                .SetRecognizer(&token)
                .map_err(|e| e.to_string())?;
            recognizer
                .SetInput(None::<&IUnknown>, true)
                .map_err(|e| format!("Microphone SAPI : {e}"))?;
            let context = recognizer.CreateRecoContext().map_err(|e| e.to_string())?;
            context.SetNotifyWin32Event().map_err(|e| e.to_string())?;
            let interest = (1u64 << SPEI_RECOGNITION.0) | (1 << 30) | (1 << 33);
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
                        .map_err(|e| format!("Phrase « {} » : {e}", trigger.phrase))?;
                }
            }
            grammar.Commit(0).map_err(|e| e.to_string())?;
            grammar
                .SetRuleState(PCWSTR::null(), std::ptr::null_mut(), SPRS_ACTIVE)
                .map_err(|e| e.to_string())?;
            recognizer
                .SetRecoState(SPRST_ACTIVE_ALWAYS)
                .map_err(|e| e.to_string())?;
            Ok(Self {
                recognizer,
                context,
                _grammar: grammar,
            })
        }
    }

    pub fn poll(&self) -> Result<Option<usize>> {
        unsafe {
            if WaitForSingleObject(self.context.GetNotifyEventHandle(), 100) != WAIT_OBJECT_0 {
                return Ok(None);
            }
            let mut matched = None;
            loop {
                let mut event = SPEVENT::default();
                let mut fetched = 0;
                self.context
                    .GetEvents(1, &mut event, &mut fetched)
                    .map_err(|e| e.to_string())?;
                if fetched == 0 {
                    break;
                }
                if event.lParam.0 != 0 && (event._bitfield >> 16) == SPET_LPARAM_IS_OBJECT.0 {
                    // GetEvents transfers one COM reference; from_raw owns and releases it.
                    let object = IUnknown::from_raw(event.lParam.0 as *mut _);
                    if event._bitfield & 0xffff == SPEI_RECOGNITION.0 {
                        let result: ISpRecoResult = object.cast().map_err(|e| e.to_string())?;
                        let phrase = result.GetPhrase().map_err(|e| e.to_string())?;
                        let id = (*phrase).Base.Rule.ulId;
                        let confidence = (*phrase).Base.Rule.Confidence;
                        CoTaskMemFree(Some(phrase.cast()));
                        if id > 0 && confidence >= 0 {
                            matched = Some(id as usize - 1);
                        }
                    }
                }
            }
            Ok(matched)
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
    }
    phrases
}
