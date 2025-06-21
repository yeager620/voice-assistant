mod audio;
mod speech;
mod voice_detection;

use anyhow::Result;
use audio::AudioCapture;
use speech::SpeechProcessor;
use voice_detection::VoiceDetector;
use serde_json::json;
use std::collections::VecDeque;
use std::time::Duration;
use tokio;

#[derive(Debug, PartialEq)]
enum ConversationState {
    Idle,
    AwaitingWakeWord,
    Listening,
    Processing,
}

struct VoiceAssistant {
    audio_capture: AudioCapture,
    speech_processor: SpeechProcessor,
    voice_detector: VoiceDetector,
    is_active: bool,
    state: ConversationState,
    command_history: VecDeque<(String, String)>,
    max_history: usize,
}

impl VoiceAssistant {
    async fn new() -> Result<Self> {
        Ok(Self {
            audio_capture: AudioCapture::new(),
            speech_processor: SpeechProcessor::new().await?,
            voice_detector: VoiceDetector::new(0.02, 0.5, "yo"),
            is_active: false,
            state: ConversationState::Idle,
            command_history: VecDeque::new(),
            max_history: 10,
        })
    }

    fn add_to_history(&mut self, command: String, response: String) {
        self.command_history.push_back((command, response));
        if self.command_history.len() > self.max_history {
            self.command_history.pop_front();
        }
    }

    async fn query_llm(&self, prompt: &str) -> Result<String> {
        let client = reqwest::Client::new();
        let response = client.post("http://localhost:11434/api/generate")
            .json(&json!({
                "model": "llama3.2:latest",
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
        match self.state {
            ConversationState::Idle | ConversationState::AwaitingWakeWord => {
                print!("\rListening for wake word... (say 'yo')\r");
                let samples = self.audio_capture.record(Duration::from_secs(2))?;

                if !samples.is_empty() {
                    self.audio_capture.save_wav(&samples, "wake_word.wav")?;
                    let wake_word_text = self.speech_processor.speech_to_text("wake_word.wav")?;

                    if self.voice_detector.matches_wake_word(&wake_word_text) {
                        println!("\nWake word detected! What can I help you with?");
                        self.state = ConversationState::Listening;
                        self.is_active = true;
                    }
                }
            },
            ConversationState::Listening => {
                println!("Listening...");
                let samples = self.audio_capture.record(Duration::from_secs(5))?;

                if samples.is_empty() || !self.voice_detector.is_voice_active(&samples, 1024) {
                    if self.voice_detector.detect_silence(&samples, 16000) {
                        println!("\nNo voice detected, returning to wake word mode...");
                        self.state = ConversationState::AwaitingWakeWord;
                        self.is_active = false;
                    }
                    return Ok(());
                }

                self.audio_capture.save_wav(&samples, "input.wav")?;
                let text = self.speech_processor.speech_to_text("input.wav")?;

                if !text.is_empty() {
                    println!("You said: {}", text);
                    self.state = ConversationState::Processing;

                    let response = self.query_llm(&text).await?;
                    println!("Assistant: {}", response);

                    self.add_to_history(text, response.clone());
                    self.speech_processor.text_to_speech(&response).await?;
                }

                self.state = ConversationState::AwaitingWakeWord;
                self.is_active = false;
            },
            ConversationState::Processing => {
                self.state = ConversationState::Listening;
            }
        }
        Ok(())
    }

    async fn run(&mut self) -> Result<()> {
        println!("Initializing Voice Assistant...");
        self.state = ConversationState::AwaitingWakeWord;

        loop {
            match self.process_interaction().await {
                Ok(_) => (),
                Err(e) => {
                    eprintln!("Error during interaction: {}", e);
                    self.state = ConversationState::AwaitingWakeWord;
                    self.is_active = false;
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
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
