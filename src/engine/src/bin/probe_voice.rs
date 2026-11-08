//! T4 probe: ElevenLabs speaks a sentence, Scribe v2 Realtime hears it back after a manual commit.
//! Set VOICE_DUMP=1 to print raw server replies to stderr (used to capture test fixtures).

use std::process::ExitCode;
use std::time::{Duration, Instant};

use aether_engine::voice::{speak, ScribeEvent, ScribeSession, SAMPLE_RATE};

const SENTENCE: &str = "Aether, send two drones to the door.";

#[tokio::main]
async fn main() -> anyhow::Result<ExitCode> {
    dotenvy::dotenv().ok();
    let dump = std::env::var("VOICE_DUMP").is_ok();

    let t = Instant::now();
    let speech = speak(SENTENCE).await?;
    let tts_ms = t.elapsed().as_millis();
    let chars = speech.chars.len();

    let mut scribe = ScribeSession::connect().await?;
    // 100 ms chunks, sent as fast as real time (docs: stream real-time audio).
    let chunk_bytes = (SAMPLE_RATE as usize / 10) * 2;
    for chunk in speech.pcm.chunks(chunk_bytes) {
        scribe.send_chunk(chunk, false).await?;
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    scribe.send_chunk(&[], true).await?;
    let commit_at = Instant::now();

    let mut result = None;
    let deadline = Duration::from_secs(20);
    while let Ok(Ok(Some((raw, ev)))) = tokio::time::timeout(deadline, scribe.next_event()).await {
        if dump {
            eprintln!("RAW {raw}");
        }
        match ev {
            ScribeEvent::Committed { text, words } if !words.is_empty() => {
                result = Some((commit_at.elapsed().as_millis(), text, words));
                break;
            }
            ScribeEvent::Error { kind, message } => {
                println!("voice_fail tts_ms={tts_ms} chars={chars} error={kind}: {message}");
                return Ok(ExitCode::FAILURE);
            }
            _ => {}
        }
    }

    let Some((stt_ms, text, words)) = result else {
        println!("voice_fail tts_ms={tts_ms} chars={chars} no committed transcript with word timestamps");
        return Ok(ExitCode::FAILURE);
    };
    let drones = words.iter().find(|w| w.word.to_lowercase().contains("drones"));
    match drones {
        Some(w) => {
            println!("voice_ok tts_ms={tts_ms} chars={chars} stt_ms={stt_ms} words={} drones_at_ms={}", words.len(), (w.start_s * 1000.0).round() as u64);
            Ok(ExitCode::SUCCESS)
        }
        None => {
            println!("voice_fail tts_ms={tts_ms} chars={chars} stt_ms={stt_ms} words={} no \"drones\" in transcript {text:?}", words.len());
            Ok(ExitCode::FAILURE)
        }
    }
}
