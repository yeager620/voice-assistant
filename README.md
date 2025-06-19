# Voice Assistant

A simple voice assistant built with Rust, using Whisper for speech recognition and text-to-speech;
The assistant listens for a wake word ("yo") and responds to user queries

## Project Tree

```
voice_assistant/
├── src/
│   ├── main.rs
│   ├── audio.rs            # Audio capture/playback
│   ├── speech.rs           # Speech processing (STT/TTS)
│   └── voice_detection.rs  # Wake word detection
├── models/ 
│   └── ggml-tiny.bin       # Whisper model
├── Cargo.toml
└── README.md               # Docs
```

## Models
LLM: LLama 2
Speech Recognition: Whisper

## System Prompt
"You are a helpful voice assistant. Keep your responses very concise and natural, like casual conversation."