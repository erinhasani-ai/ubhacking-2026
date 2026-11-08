//! ElevenLabs speech out and Scribe speech in (T4).
//! Docs: elevenlabs.io/docs/api-reference/text-to-speech/convert-with-timestamps and
//! elevenlabs.io/docs/api-reference/speech-to-text/v-1-speech-to-text-realtime

use anyhow::{anyhow, bail, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::{client::IntoClientRequest, Message};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

/// Sample rate for both directions: TTS `pcm_16000` is what Scribe `pcm_16000` accepts, so no resampling.
pub const SAMPLE_RATE: u32 = 16_000;

#[derive(Debug, Clone, PartialEq)]
pub struct CharTiming {
    pub ch: String,
    pub start_s: f64,
    pub end_s: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WordTiming {
    pub word: String,
    pub start_s: f64,
    pub end_s: f64,
}

/// One spoken sentence: raw 16-bit little-endian mono PCM at `SAMPLE_RATE`, plus per-character timings.
#[derive(Debug, Clone)]
pub struct Speech {
    pub pcm: Vec<u8>,
    pub chars: Vec<CharTiming>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScribeEvent {
    Started,
    Partial { text: String },
    /// `words` is empty for a plain `committed_transcript`; filled for `committed_transcript_with_timestamps`.
    Committed { text: String, words: Vec<WordTiming> },
    Error { kind: String, message: String },
    Other(String),
}

#[derive(Deserialize)]
struct Alignment {
    characters: Vec<String>,
    character_start_times_seconds: Vec<f64>,
    character_end_times_seconds: Vec<f64>,
}

#[derive(Deserialize)]
struct TtsReply {
    audio_base64: String,
    alignment: Option<Alignment>,
}

/// Parses the `with-timestamps` reply body into PCM bytes and character timings.
pub fn parse_tts_reply(body: &str) -> Result<Speech> {
    let reply: TtsReply = serde_json::from_str(body).context("TTS reply is not the expected JSON")?;
    let a = reply.alignment.ok_or_else(|| anyhow!("TTS reply has no alignment"))?;
    if a.characters.len() != a.character_start_times_seconds.len() || a.characters.len() != a.character_end_times_seconds.len() {
        bail!("TTS alignment arrays differ in length");
    }
    let chars = a
        .characters
        .into_iter()
        .zip(a.character_start_times_seconds)
        .zip(a.character_end_times_seconds)
        .map(|((ch, start_s), end_s)| CharTiming { ch, start_s, end_s })
        .collect();
    let pcm = STANDARD.decode(reply.audio_base64).context("TTS audio is not base64")?;
    Ok(Speech { pcm, chars })
}

/// Parses one Scribe Realtime server message.
pub fn parse_scribe_event(msg: &str) -> Result<ScribeEvent> {
    let v: Value = serde_json::from_str(msg).context("Scribe message is not JSON")?;
    let kind = v["message_type"].as_str().ok_or_else(|| anyhow!("Scribe message has no message_type"))?;
    let text = || v["text"].as_str().unwrap_or_default().to_string();
    Ok(match kind {
        "session_started" => ScribeEvent::Started,
        "partial_transcript" => ScribeEvent::Partial { text: text() },
        "committed_transcript" => ScribeEvent::Committed { text: text(), words: vec![] },
        "committed_transcript_with_timestamps" => {
            let words = v["words"]
                .as_array()
                .map(|ws| {
                    ws.iter()
                        .filter(|w| w["type"] == "word")
                        .filter_map(|w| {
                            Some(WordTiming { word: w["text"].as_str()?.to_string(), start_s: w["start"].as_f64()?, end_s: w["end"].as_f64()? })
                        })
                        .collect()
                })
                .unwrap_or_default();
            ScribeEvent::Committed { text: text(), words }
        }
        k if v.get("error").is_some() => ScribeEvent::Error { kind: k.to_string(), message: v["error"].as_str().unwrap_or_default().to_string() },
        k => ScribeEvent::Other(k.to_string()),
    })
}

/// Speaks one sentence. Reads `ELEVENLABS_API_KEY` and `ELEVENLABS_VOICE_ID`; the key goes only in a header.
pub async fn speak(sentence: &str) -> Result<Speech> {
    let key = std::env::var("ELEVENLABS_API_KEY").context("ELEVENLABS_API_KEY not set")?;
    let voice = std::env::var("ELEVENLABS_VOICE_ID").context("ELEVENLABS_VOICE_ID not set")?;
    let url = format!("https://api.elevenlabs.io/v1/text-to-speech/{voice}/with-timestamps?output_format=pcm_16000");
    let res = reqwest::Client::new()
        .post(url)
        .header("xi-api-key", key)
        .json(&json!({ "text": sentence }))
        .send()
        .await
        .map_err(|e| anyhow!("TTS request failed: {}", e.without_url()))?;
    let status = res.status();
    let body = res.text().await.map_err(|e| anyhow!("TTS body read failed: {}", e.without_url()))?;
    if !status.is_success() {
        bail!("TTS HTTP {status}: {body}");
    }
    parse_tts_reply(&body)
}

type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// A Scribe v2 Realtime session with manual commit and word timestamps.
pub struct ScribeSession {
    ws: Ws,
}

impl ScribeSession {
    pub async fn connect() -> Result<Self> {
        let key = std::env::var("ELEVENLABS_API_KEY").context("ELEVENLABS_API_KEY not set")?;
        let url = "wss://api.elevenlabs.io/v1/speech-to-text/realtime?model_id=scribe_v2_realtime&audio_format=pcm_16000&commit_strategy=manual&include_timestamps=true";
        let mut req = url.into_client_request()?;
        req.headers_mut().insert("xi-api-key", key.parse().map_err(|_| anyhow!("API key is not a valid header value"))?);
        let (ws, _) = tokio_tungstenite::connect_async(req).await.map_err(|e| anyhow!("Scribe connect failed: {e}"))?;
        Ok(Self { ws })
    }

    /// Sends PCM (16-bit LE mono, `SAMPLE_RATE`); `commit` asks Scribe to finalise the transcript.
    pub async fn send_chunk(&mut self, pcm: &[u8], commit: bool) -> Result<()> {
        let m = json!({ "message_type": "input_audio_chunk", "audio_base_64": STANDARD.encode(pcm), "commit": commit, "sample_rate": SAMPLE_RATE });
        self.ws.send(Message::text(m.to_string())).await.map_err(|e| anyhow!("Scribe send failed: {e}"))
    }

    /// Next parsed server event; `None` when the socket closes. Raw text is returned too, for logging fixtures.
    pub async fn next_event(&mut self) -> Result<Option<(String, ScribeEvent)>> {
        while let Some(m) = self.ws.next().await {
            if let Message::Text(t) = m.map_err(|e| anyhow!("Scribe read failed: {e}"))? {
                let raw = t.to_string();
                let ev = parse_scribe_event(&raw)?;
                return Ok(Some((raw, ev)));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Real reply to "Go now." (pcm_16000, with-timestamps), captured live 2026-11-07.
    // EDITED: audio_base64 cut to its first 16 chars; normalized_alignment and quality_check removed.
    const TTS: &str = r#"{"audio_base64":"AQABAP//AQABAAEA","alignment":{"characters":["G","o"," ","n","o","w","."],"character_start_times_seconds":[0.0,0.139,0.267,0.348,0.395,0.522,0.604],"character_end_times_seconds":[0.139,0.267,0.348,0.395,0.522,0.604,0.789]}}"#;

    // Real Scribe replies from the probe, captured live 2026-11-07. Unedited.
    const PARTIAL: &str = r#"{"message_type":"partial_transcript","text":"Ether, send 2 d"}"#;
    const STARTED_PREFIX: &str = r#"{"message_type":"session_started","session_id":"9f35c4fb3b344ef5bb16ee955ca745eb","config":{"sample_rate":16000}}"#;
    // Real committed_transcript_with_timestamps. EDITED: words cut after "drones" and the per-character arrays removed.
    const COMMITTED: &str = r#"{"message_type":"committed_transcript_with_timestamps","text":"Ether, send two drones to the door.","language_code":null,"words":[{"text":"Ether,","start":0.12,"end":0.48,"type":"word","speaker_id":null,"logprob":-0.5715459187825521,"channel_index":null},{"text":" ","start":0.48,"end":0.96,"type":"spacing","speaker_id":null,"logprob":-0.5698280334472656,"channel_index":null},{"text":"send","start":0.96,"end":1.2,"type":"word","speaker_id":null,"logprob":-0.7991170883178711,"channel_index":null},{"text":" ","start":1.2,"end":1.26,"type":"spacing","speaker_id":null,"logprob":-0.5836124420166016,"channel_index":null},{"text":"two","start":1.26,"end":1.38,"type":"word","speaker_id":null,"logprob":-0.5223026275634766,"channel_index":null},{"text":" ","start":1.38,"end":1.48,"type":"spacing","speaker_id":null,"logprob":-0.9169540405273438,"channel_index":null},{"text":"drones","start":1.48,"end":1.98,"type":"word","speaker_id":null,"logprob":-0.8268556594848633,"channel_index":null}]}"#;
    // HAND-WRITTEN from the docs' error shape ({message_type, error}); not captured live.
    const ERROR: &str = r#"{"message_type":"auth_error","error":"Invalid API key or token."}"#;

    #[test]
    fn tts_alignment_gives_exact_character_timings() {
        let s = parse_tts_reply(TTS).unwrap();
        assert_eq!(s.pcm, vec![1, 0, 1, 0, 255, 255, 1, 0, 1, 0, 1, 0]);
        assert_eq!(s.chars.len(), 7);
        assert_eq!(s.chars[0], CharTiming { ch: "G".into(), start_s: 0.0, end_s: 0.139 });
        assert_eq!(s.chars[2].ch, " ");
        assert_eq!(s.chars[6], CharTiming { ch: ".".into(), start_s: 0.604, end_s: 0.789 });
    }

    #[test]
    fn tts_without_alignment_or_with_bad_json_is_an_error() {
        assert!(parse_tts_reply(r#"{"audio_base64":"AAAA","alignment":null}"#).is_err());
        assert!(parse_tts_reply("{nope").is_err());
    }

    #[test]
    fn partial_transcript_parses() {
        assert_eq!(parse_scribe_event(PARTIAL).unwrap(), ScribeEvent::Partial { text: "Ether, send 2 d".into() });
        assert_eq!(parse_scribe_event(STARTED_PREFIX).unwrap(), ScribeEvent::Started);
    }

    #[test]
    fn committed_transcript_gives_words_with_times_and_skips_spacing() {
        let ScribeEvent::Committed { text, words } = parse_scribe_event(COMMITTED).unwrap() else { panic!("not Committed") };
        assert_eq!(text, "Ether, send two drones to the door.");
        let w = |word: &str, start_s, end_s| WordTiming { word: word.into(), start_s, end_s };
        assert_eq!(words, vec![w("Ether,", 0.12, 0.48), w("send", 0.96, 1.2), w("two", 1.26, 1.38), w("drones", 1.48, 1.98)]);
    }

    #[test]
    fn error_message_parses_to_error() {
        assert_eq!(parse_scribe_event(ERROR).unwrap(), ScribeEvent::Error { kind: "auth_error".into(), message: "Invalid API key or token.".into() });
    }

    #[test]
    fn malformed_json_is_err_not_panic() {
        assert!(parse_scribe_event("{\"message_type\":").is_err());
        assert!(parse_scribe_event("[]").is_err());
        assert!(parse_scribe_event("").is_err());
    }
}
