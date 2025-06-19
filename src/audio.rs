use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::mpsc;
use std::time::Duration;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const NOISE_GATE_THRESHOLD: f32 = 0.02;

pub struct AudioCapture {
    host: cpal::Host,
    recording: Arc<AtomicBool>,
}

impl AudioCapture {
    pub fn new() -> Self {
        Self {
            host: cpal::default_host(),
            recording: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start_recording(&self) {
        self.recording.store(true, Ordering::SeqCst);
    }

    pub fn stop_recording(&self) {
        self.recording.store(false, Ordering::SeqCst);
    }

    pub fn is_recording(&self) -> bool {
        self.recording.load(Ordering::SeqCst)
    }

    fn apply_noise_gate(sample: f32) -> f32 {
        if sample.abs() < NOISE_GATE_THRESHOLD {
            0.0
        } else {
            sample
        }
    }

    fn get_supported_config(device: &cpal::Device) -> Result<cpal::StreamConfig> {
        let default_config = device.default_input_config()?;
        println!("Default input config: {:?}", default_config);

        let config = cpal::StreamConfig {
            channels: 1,
            sample_rate: default_config.sample_rate(),
            buffer_size: cpal::BufferSize::Default,
        };

        Ok(config)
    }

    pub fn record(&self, duration: Duration) -> Result<Vec<f32>> {
        let device = self.host.default_input_device()
            .ok_or(anyhow::anyhow!("No input device available"))?;

        let config = Self::get_supported_config(&device)?;
        println!("Using audio config: {:?}", config);

        let (tx, rx) = mpsc::channel::<f32>();
        let recording = self.recording.clone();
        let err_fn = |err| eprintln!("An error occurred on stream: {}", err);

        let stream = device.build_input_stream(
            &config,
            move |data: &[f32], _: &_| {
                if recording.load(Ordering::SeqCst) {
                    for &sample in data {
                        let processed = Self::apply_noise_gate(sample);
                        let _ = tx.send(processed);
                    }
                }
            },
            err_fn,
            None,
        )?;

        stream.play()?;
        self.start_recording();
        std::thread::sleep(duration);
        self.stop_recording();
        drop(stream);

        let samples: Vec<f32> = rx.try_iter().collect();
        let original_rate = config.sample_rate.0;

        if original_rate != 16000 {
            let ratio = 16000.0 / original_rate as f32;
            let out_len = (samples.len() as f32 * ratio) as usize;
            let mut resampled = Vec::with_capacity(out_len);

            for i in 0..out_len {
                let pos = i as f32 / ratio;
                let pos_floor = pos.floor() as usize;
                if pos_floor >= samples.len() - 1 {
                    break;
                }
                let fract = pos - pos_floor as f32;
                let s1 = samples[pos_floor];
                let s2 = samples[pos_floor + 1];
                resampled.push(s1 * (1.0 - fract) + s2 * fract);
            }

            Ok(resampled)
        } else {
            Ok(samples)
        }
    }

    pub fn save_wav(&self, samples: &[f32], path: &str) -> Result<()> {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 16000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = hound::WavWriter::create(path, spec)?;

        for &sample in samples {
            writer.write_sample((sample * i16::MAX as f32) as i16)?;
        }

        writer.finalize()?;
        Ok(())
    }
}
