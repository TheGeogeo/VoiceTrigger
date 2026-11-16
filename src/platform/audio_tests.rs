use super::audio::Microphone;
use std::time::{Duration, Instant};

#[test]
#[ignore = "Opens the real default microphone; run explicitly on the Windows target"]
fn default_microphone_produces_valid_audio_buffers() {
    let mut microphone = Microphone::open().expect("Default microphone must open");
    microphone.start().expect("Capture must start");
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut received = 0;
    while Instant::now() < deadline && received < 4 {
        if let Some((rms, duration)) = microphone
            .next_level()
            .expect("Audio buffer must be readable")
        {
            assert!(rms.is_finite() && (0.0..=1.0).contains(&rms));
            assert_eq!(duration, Duration::from_millis(50));
            received += 1;
        } else {
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    assert_eq!(received, 4, "Four buffers must arrive before the deadline");
}
