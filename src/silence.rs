use std::time::Duration;

pub const SILENCE: Duration = Duration::from_secs(3);
const WAIT_FOR_VOICE: Duration = Duration::from_secs(15);
const MAX_RECORDING: Duration = Duration::from_secs(120);

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Listening,
    Submit,
    Cancel,
}

pub struct SilenceDetector {
    last_voice: Option<Duration>,
    voiced: Duration,
}

impl SilenceDetector {
    pub fn new() -> Self {
        Self {
            last_voice: None,
            voiced: Duration::ZERO,
        }
    }

    /// `now` is elapsed recording time; `chunk` is this audio buffer's duration.
    pub fn update(&mut self, now: Duration, chunk: Duration, voice: bool) -> Outcome {
        if now >= MAX_RECORDING {
            return Outcome::Cancel;
        }
        if voice {
            self.voiced += chunk;
            if self.last_voice.is_some() || self.voiced >= Duration::from_millis(150) {
                self.last_voice = Some(now);
            }
        } else {
            self.voiced = Duration::ZERO;
        }
        match self.last_voice {
            Some(last) if now.saturating_sub(last) >= SILENCE => Outcome::Submit,
            None if now >= WAIT_FOR_VOICE => Outcome::Cancel,
            _ => Outcome::Listening,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn ms(value: u64) -> Duration {
        Duration::from_millis(value)
    }

    #[test]
    fn waits_three_seconds_after_speech_and_resets_on_new_speech() {
        let mut detector = SilenceDetector::new();
        assert_eq!(detector.update(ms(200), ms(200), true), Outcome::Listening);
        assert_eq!(detector.update(ms(3199), ms(50), false), Outcome::Listening);
        detector.update(ms(3200), ms(200), true);
        assert_eq!(detector.update(ms(6199), ms(50), false), Outcome::Listening);
        assert_eq!(detector.update(ms(6200), ms(50), false), Outcome::Submit);
    }

    #[test]
    fn empty_recording_and_short_click_never_submit() {
        let mut detector = SilenceDetector::new();
        detector.update(ms(50), ms(50), true);
        assert_eq!(detector.update(ms(3000), ms(50), false), Outcome::Listening);
        assert_eq!(detector.update(ms(15000), ms(50), false), Outcome::Cancel);
    }

    #[test]
    fn long_recording_cancels_instead_of_sending_incomplete_text() {
        let mut detector = SilenceDetector::new();
        detector.update(ms(200), ms(200), true);
        assert_eq!(detector.update(ms(120000), ms(200), true), Outcome::Cancel);
    }

    #[test]
    fn resumed_speech_immediately_prevents_submission() {
        let mut detector = SilenceDetector::new();
        detector.update(ms(200), ms(200), true);
        assert_eq!(detector.update(ms(3200), ms(50), true), Outcome::Listening);
        assert_eq!(detector.update(ms(6199), ms(50), false), Outcome::Listening);
        assert_eq!(detector.update(ms(6200), ms(50), false), Outcome::Submit);
    }
}
