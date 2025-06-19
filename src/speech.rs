use anyhow::Result;
use whisper_rs::{WhisperContext, FullParams, SamplingStrategy};
use std::path::Path;
use std::fs;
use reqwest;
use std::io::{copy, Cursor};
use rodio::{Decoder, OutputStream, Sink};

const MODEL_URL: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin";
const TTS_URL: &str = "https://translate.google.com/translate_tts";

pub struct SpeechProcessor {
    whisper_ctx: WhisperContext,
}

impl SpeechProcessor {
    async fn download_model(path: &str) -> Result<()> {
        println!("Downloading Whisper model...");
        let response = reqwest::get(MODEL_URL).await?;
        let mut file = fs::File::create(path)?;
        copy(&mut response.bytes().await?.as_ref(), &mut file)?;
        println!("Model downloaded successfully");
        Ok(())
    }

    pub async fn new() -> Result<Self> {
        let model_path = "models/ggml-tiny.bin";
        if !Path::new(model_path).exists() {
            Self::download_model(model_path).await?;
        }

        let ctx = WhisperContext::new_with_params(model_path, Default::default())?;

        Ok(Self {
            whisper_ctx: ctx,
        })
    }

    pub fn speech_to_text(&self, audio_path: &str) -> Result<String> {
        let audio_data = self.load_audio(audio_path)?;
        let mut state = self.whisper_ctx.create_state()?;
        
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 3 });
        params.set_translate(false);
        params.set_language(Some("en"));
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_timestamps(false);
        
        state.full(params, &audio_data[..])?;
        
        let num_segments = state.full_n_segments()?;
        let mut text = String::new();
        
        for i in 0..num_segments {
            if let Ok(segment) = state.full_get_segment_text(i) {
                let cleaned = segment.replace("[noise]", "")
                                   .replace("[silence]", "")
                                   .trim()
                                   .to_string();

                if !cleaned.is_empty() {
                    text.push_str(&cleaned);
                    text.push(' ');
                }
            }
        }

        Ok(text.trim().to_string())
    }

    fn load_audio(&self, path: &str) -> Result<Vec<f32>> {
        let mut reader = hound::WavReader::open(path)?;
        let samples: Vec<f32> = reader.samples::<i16>()
            .filter_map(Result::ok)
            .map(|s| s as f32 / i16::MAX as f32)
            .collect();
        Ok(samples)
    }

    pub async fn text_to_speech(&self, text: &str) -> Result<()> {
        let chunks: Vec<&str> = text.split(|c| c == '.' || c == '?' || c == '!')
            .filter(|s| !s.trim().is_empty())
            .collect();

        let client = reqwest::Client::new();
        let (_stream, stream_handle) = OutputStream::try_default()?;
        let sink = Sink::try_new(&stream_handle)?;

        for chunk in chunks {
            let text = chunk.trim();
            if text.is_empty() {
                continue;
            }

            let response = client.get(TTS_URL)
                .query(&[
                    ("ie", "UTF-8"),
                    ("tl", "en"),
                    ("q", text),
                    ("client", "tw-ob"),
                ])
                .header("User-Agent", "Mozilla/5.0")
                .send()
                .await?;

            let audio_data = response.bytes().await?;
            let cursor = Cursor::new(audio_data);

            let source = Decoder::new(cursor)?;
            sink.append(source);
        }

        sink.sleep_until_end();
        Ok(())
    }
}
