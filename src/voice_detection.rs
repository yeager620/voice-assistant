use anyhow::Result;
use std::collections::VecDeque;

pub struct VoiceDetector {
    energy_threshold: f32,
    silence_duration: f32,
    activation_word: String,
    noise_floor: f32,
    energy_history: VecDeque<f32>,
    history_size: usize,
    consecutive_threshold: usize,
}

impl VoiceDetector {
    pub fn new(energy_threshold: f32, silence_duration: f32, activation_word: &str) -> Self {
        Self {
            energy_threshold,
            silence_duration,
            activation_word: activation_word.to_lowercase(),
            noise_floor: 0.01,
            energy_history: VecDeque::with_capacity(50),
            history_size: 50,
            consecutive_threshold: 3,
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

    fn update_noise_floor(&mut self, rms: f32) {
        self.energy_history.push_back(rms);
        if self.energy_history.len() > self.history_size {
            self.energy_history.pop_front();
        }

        if !self.energy_history.is_empty() {
            let mut energies: Vec<f32> = self.energy_history.iter().copied().collect();
            energies.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let index = (energies.len() as f32 * 0.1) as usize;
            self.noise_floor = energies[index.max(1) - 1];
        }
    }

    pub fn is_voice_active(&mut self, samples: &[f32], window_size: usize) -> bool {
        let mut consecutive_active = 0;
        let required_consecutive = self.consecutive_threshold;

        for chunk in samples.chunks(window_size) {
            let rms = Self::calculate_rms(chunk);
            self.update_noise_floor(rms);

            let dynamic_threshold = self.noise_floor * 2.5;
            let threshold = self.energy_threshold.max(dynamic_threshold);

            if rms > threshold {
                consecutive_active += 1;
                if consecutive_active >= required_consecutive {
                    return true;
                }
            } else {
                consecutive_active = 0;
            }
        }
        false
    }

    pub fn detect_silence(&mut self, samples: &[f32], sample_rate: u32) -> bool {
        let window_size = (sample_rate as f32 * self.silence_duration) as usize;
        !self.is_voice_active(samples, window_size)
    }

    fn levenshtein_distance(s1: &str, s2: &str) -> usize {
        let len1 = s1.chars().count();
        let len2 = s2.chars().count();

        let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

        for i in 0..=len1 {
            matrix[i][0] = i;
        }
        for j in 0..=len2 {
            matrix[0][j] = j;
        }

        for (i, c1) in s1.chars().enumerate() {
            for (j, c2) in s2.chars().enumerate() {
                let cost = if c1 == c2 { 0 } else { 1 };
                matrix[i + 1][j + 1] = (matrix[i][j + 1] + 1)
                    .min(matrix[i + 1][j] + 1)
                    .min(matrix[i][j] + cost);
            }
        }

        matrix[len1][len2]
    }

    pub fn matches_wake_word(&self, text: &str) -> bool {
        let text = text.to_lowercase();
        let words: Vec<&str> = text.split_whitespace().collect();

        if text.contains(&self.activation_word) {
            return true;
        }

        for word in words {
            if word.len() >= 2 &&
               Self::levenshtein_distance(word, &self.activation_word) <= 1 {
                return true;
            }
        }

        text.contains("yo") ||
        text.contains("yoo") ||
        text.contains("you") ||
        text.contains("yeah")
    }
}
