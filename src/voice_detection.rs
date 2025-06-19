use anyhow::Result;

pub struct VoiceDetector {
    energy_threshold: f32,
    silence_duration: f32,
    activation_word: String,
}

impl VoiceDetector {
    pub fn new(energy_threshold: f32, silence_duration: f32, activation_word: &str) -> Self {
        Self {
            energy_threshold,
            silence_duration,
            activation_word: activation_word.to_lowercase(),
        }
    }

    pub fn calculate_rms(samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        
        let sum_squares: f32 = samples.iter()
            .map(|&sample| sample * sample)
            .sum();
            
        (sum_squares / samples.len() as f32).sqrt()
    }

    pub fn is_voice_active(&self, samples: &[f32], window_size: usize) -> bool {
        samples.chunks(window_size)
            .map(Self::calculate_rms)
            .any(|rms| rms > self.energy_threshold)
    }

    pub fn detect_silence(&self, samples: &[f32], sample_rate: u32) -> bool {
        let window_size = (sample_rate as f32 * self.silence_duration) as usize;
        !self.is_voice_active(samples, window_size)
    }

    pub fn matches_wake_word(&self, text: &str) -> bool {
        let text = text.to_lowercase();
        text.contains(&self.activation_word) || 
        text.contains("you") ||
        text.contains("yeah")
    }
}
