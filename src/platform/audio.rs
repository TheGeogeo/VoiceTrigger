use super::Result;
use std::time::Duration;
use windows::core::PSTR;
use windows::Win32::Media::Audio::*;

const SAMPLE_RATE: u32 = 44100;
const SAMPLES: usize = SAMPLE_RATE as usize / 20;
const HEADER_SIZE: u32 = std::mem::size_of::<WAVEHDR>() as u32;

struct Buffer {
    samples: Box<[i16; SAMPLES]>,
    header: Box<WAVEHDR>,
    prepared: bool,
}

pub struct Microphone {
    handle: HWAVEIN,
    buffers: Vec<Buffer>,
    next: usize,
}

fn check(code: u32) -> Result<()> {
    if code == 0 {
        return Ok(());
    }
    let mut text = [0u16; 256];
    unsafe {
        waveInGetErrorTextW(code, &mut text);
    }
    let length = text.iter().position(|c| *c == 0).unwrap_or(text.len());
    Err(format!(
        "Microphone: {} (code {code}). Check permissions and the default input device.",
        String::from_utf16_lossy(&text[..length])
    ))
}

impl Microphone {
    pub fn open() -> Result<Self> {
        unsafe {
            let format = WAVEFORMATEX {
                wFormatTag: 1,
                nChannels: 1,
                nSamplesPerSec: SAMPLE_RATE,
                nAvgBytesPerSec: SAMPLE_RATE * 2,
                nBlockAlign: 2,
                wBitsPerSample: 16,
                cbSize: 0,
            };
            let mut handle = HWAVEIN::default();
            check(waveInOpen(
                Some(&mut handle),
                WAVE_MAPPER,
                &format,
                0,
                0,
                MIDI_WAVE_OPEN_TYPE(0),
            ))?;
            let mut microphone = Self {
                handle,
                buffers: Vec::new(),
                next: 0,
            };
            for _ in 0..8 {
                let mut samples = Box::new([0i16; SAMPLES]);
                let header = Box::new(WAVEHDR {
                    lpData: PSTR(samples.as_mut_ptr().cast()),
                    dwBufferLength: (SAMPLES * 2) as u32,
                    ..Default::default()
                });
                microphone.buffers.push(Buffer {
                    samples,
                    header,
                    prepared: false,
                });
                let buffer = microphone.buffers.last_mut().unwrap();
                check(waveInPrepareHeader(
                    handle,
                    buffer.header.as_mut(),
                    HEADER_SIZE,
                ))?;
                buffer.prepared = true;
                check(waveInAddBuffer(handle, buffer.header.as_mut(), HEADER_SIZE))?;
            }
            Ok(microphone)
        }
    }

    pub fn start(&self) -> Result<()> {
        unsafe { check(waveInStart(self.handle)) }
    }

    pub fn next_level(&mut self) -> Result<Option<(f32, Duration)>> {
        let buffer = &mut self.buffers[self.next];
        unsafe {
            // The native audio driver writes these fields asynchronously.
            let flags = std::ptr::read_volatile(std::ptr::addr_of!(buffer.header.dwFlags));
            if flags & WHDR_DONE == 0 {
                return Ok(None);
            }
            let count = (std::ptr::read_volatile(std::ptr::addr_of!(buffer.header.dwBytesRecorded))
                / 2) as usize;
            let count = count.min(SAMPLES);
            let energy = buffer.samples[..count]
                .iter()
                .map(|sample| (*sample as f32 / 32768.0).powi(2))
                .sum::<f32>();
            let rms = (energy / count.max(1) as f32).sqrt();
            check(waveInAddBuffer(
                self.handle,
                buffer.header.as_mut(),
                HEADER_SIZE,
            ))?;
            self.next = (self.next + 1) % self.buffers.len();
            Ok(Some((
                rms,
                Duration::from_secs_f64(count as f64 / SAMPLE_RATE as f64),
            )))
        }
    }
}

impl Drop for Microphone {
    fn drop(&mut self) {
        unsafe {
            waveInReset(self.handle);
            for buffer in &mut self.buffers {
                if buffer.prepared {
                    waveInUnprepareHeader(self.handle, buffer.header.as_mut(), HEADER_SIZE);
                }
            }
            waveInClose(self.handle);
        }
    }
}
