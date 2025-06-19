mod audio;
mod speech;
mod voice_detection;

use anyhow::Result;
use audio::AudioCapture;
use speech::SpeechProcessor;
use voice_detection::VoiceDetector;
use serde_json::json;
use std::time::Duration;
use tokio;

struct VoiceAssistant {
    audio_capture: AudioCapture,
    speech_processor: SpeechProcessor,
    voice_detector: VoiceDetector,
    is_active: bool,
}

impl VoiceAssistant {
    async fn new() -> Result<Self> {
        Ok(Self {
            audio_capture: AudioCapture::new(),
            speech_processor: SpeechProcessor::new().await?,
            voice_detector: VoiceDetector::new(0.02, 0.5, "yo"),
            is_active: false,
        })
    }

    async fn query_llm(&self, prompt: &str) -> Result<String> {
        let client = reqwest::Client::new();
        let response = client.post("http://localhost:11434/api/generate")
            .json(&json!({
                "model": "llama2",
                "prompt": prompt,
                "stream": false,
                "system": "You are a helpful voice assistant. Keep your responses clear, concise, and natural. Use only words in your response, no emojis."
            }))
            .send()
            .await?;

        let json: serde_json::Value = response.json().await?;
        
        json.get("response")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("Invalid response format from Ollama"))
    }

    async fn process_interaction(&mut self) -> Result<()> {
        if !self.is_active {
            print!("\rListening for wake word... (say 'yo')\r");
            let samples = self.audio_capture.record(Duration::from_secs(2))?;
            
            if !samples.is_empty() {
                self.audio_capture.save_wav(&samples, "wake_word.wav")?;
                let wake_word_text = self.speech_processor.speech_to_text("wake_word.wav")?;
                
                if self.voice_detector.matches_wake_word(&wake_word_text) {
                    println!("\nWake word detected! What can I help you with?");
                    self.is_active = true;
                    return Ok(());
                }
            }
            return Ok(());
        }

        println!("Listening...");
        let samples = self.audio_capture.record(Duration::from_secs(5))?;
        
        if samples.is_empty() || !self.voice_detector.is_voice_active(&samples, 1024) {
            if self.voice_detector.detect_silence(&samples, 16000) {
                println!("\nNo voice detected, returning to wake word mode...");
                self.is_active = false;
            }
            return Ok(());
        }

        self.audio_capture.save_wav(&samples, "input.wav")?;
        let text = self.speech_processor.speech_to_text("input.wav")?;
        
        if text.is_empty() {
            return Ok(());
        }

        println!("You said: {}", text);

        let response = self.query_llm(&text).await?;
        println!("Assistant: {}", response);

        self.speech_processor.text_to_speech(&response).await?;

        self.is_active = false;
        Ok(())
    }

    async fn run(&mut self) -> Result<()> {
        println!("Initializing Voice Assistant...");

        loop {
            if let Err(e) = self.process_interaction().await {
                eprintln!("Error: {}", e);
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut assistant = VoiceAssistant::new().await?;
    println!("voice assistant started; say 'yo' to begin");
    assistant.run().await
}
