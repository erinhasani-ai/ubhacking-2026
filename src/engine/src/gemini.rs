//! Gemini function call that turns speech into a Command. The reply maps to wire JSON, and only aether-core parses it.

use aether_core::command::Command;
use serde_json::{json, Value};

const MODELS_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models";

const SYSTEM: &str = "You are Aether, the command parser of a rescue drone swarm. Turn what a rescue commander says into exactly one function call. If what was said is not a command for the swarm, make no call. Pick drones with drone_count or drone_ids; neither means all drones. For dispatch, give place as a lowercase noun without an article (for example door), or set there to true when the speaker says there or here while pointing.";

fn drone_params() -> Value {
    json!({
        "drone_count": {"type": "INTEGER", "description": "How many drones, 1 to 6."},
        "drone_ids": {"type": "ARRAY", "items": {"type": "INTEGER"}, "description": "Which drones, each 1 to 6."}
    })
}

fn declarations() -> Value {
    let mut dispatch = drone_params();
    dispatch["place"] = json!({"type": "STRING", "description": "A named place: a lowercase noun without an article, for example door."});
    dispatch["there"] = json!({"type": "BOOLEAN", "description": "True when the speaker says there or here while pointing."});
    json!([
        {"name": "dispatch", "description": "Send Drones from the Launch pad to a target.",
         "parameters": {"type": "OBJECT", "properties": dispatch}},
        {"name": "advance", "description": "Move the Drones one step forward in the view, one metre along the map view's horizontal forward.",
         "parameters": {"type": "OBJECT", "properties": drone_params()}},
        {"name": "recall", "description": "Fly the Drones back to the Launch pad and land them.",
         "parameters": {"type": "OBJECT", "properties": drone_params()}}
    ])
}

/// Sends one transcript to Gemini and returns the raw generateContent reply.
pub async fn generate(client: &reqwest::Client, api_key: &str, model: &str, transcript: &str) -> anyhow::Result<Value> {
    let body = json!({
        "systemInstruction": {"parts": [{"text": SYSTEM}]},
        "contents": [{"role": "user", "parts": [{"text": transcript}]}],
        "tools": [{"functionDeclarations": declarations()}],
        // MINIMAL is the lowest thinking level the docs list for gemini-3.6-flash (lowest latency).
        "generationConfig": {"thinkingConfig": {"thinkingLevel": "MINIMAL"}}
    });
    // The key travels only in the header: reqwest error text prints the URL.
    let resp = client
        .post(format!("{MODELS_URL}/{model}:generateContent"))
        .header("x-goog-api-key", api_key)
        .json(&body)
        .send()
        .await?;
    let status = resp.status();
    let text = resp.text().await?;
    if !status.is_success() {
        let message = serde_json::from_str::<Value>(&text)
            .ok()
            .and_then(|v| v["error"]["message"].as_str().map(str::to_owned))
            .unwrap_or(text);
        anyhow::bail!("Gemini returned {status}: {message}");
    }
    Ok(serde_json::from_str(&text)?)
}

/// Maps a reply to a Command. No function call gives None; any call must parse as a Command.
pub fn to_command(reply: &Value) -> Result<Option<Command>, String> {
    let parts = reply.pointer("/candidates/0/content/parts").and_then(Value::as_array);
    let calls: Vec<&Value> = parts.into_iter().flatten().filter_map(|p| p.get("functionCall")).collect();
    match calls.as_slice() {
        [] => Ok(None),
        [call] => {
            let name = call["name"].as_str().ok_or("functionCall has no name")?;
            let args = call.get("args").cloned().unwrap_or_else(|| json!({}));
            Command::from_json(&wire(name, &args)?).map(Some)
        }
        _ => Err(format!("{} function calls in one reply, expected one", calls.len())),
    }
}

/// Maps a function name and its flat arguments to the Command wire JSON.
fn wire(name: &str, args: &Value) -> Result<String, String> {
    let obj = args.as_object().ok_or("functionCall args is not an object")?;
    let allowed: &[&str] = match name {
        "dispatch" => &["drone_count", "drone_ids", "place", "there"],
        "advance" | "recall" => &["drone_count", "drone_ids"],
        other => return Err(format!("no function named {other:?}")),
    };
    if let Some(key) = obj.keys().find(|k| !allowed.contains(&k.as_str())) {
        return Err(format!("{name} got unknown argument {key:?}"));
    }
    let drones = match (obj.get("drone_count"), obj.get("drone_ids")) {
        (Some(_), Some(_)) => return Err("both drone_count and drone_ids given".into()),
        (Some(count), None) => json!({"count": count}),
        (None, Some(ids)) => json!({"ids": ids}),
        (None, None) => json!("all"),
    };
    if name != "dispatch" {
        return Ok(json!({"kind": name, "drones": drones}).to_string());
    }
    let there = match obj.get("there") {
        None => false,
        Some(v) => v.as_bool().ok_or("there must be true or false")?,
    };
    let target = match (there, obj.get("place")) {
        (true, Some(_)) => return Err("both there and place given".into()),
        (true, None) => json!("there"),
        (false, Some(place)) => json!({"place": normalise_place(place.as_str().ok_or("place must be a string")?)}),
        (false, None) => return Err("dispatch needs a place or there".into()),
    };
    Ok(json!({"kind": "dispatch", "drones": drones, "target": target}).to_string())
}

/// "The Door " becomes "door".
fn normalise_place(raw: &str) -> String {
    let lower = raw.trim().to_lowercase();
    lower.strip_prefix("the ").unwrap_or(&lower).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Trimmed copies of live generateContent replies from gemini-3.6-flash (thoughtSignature and usage removed).
    const DISPATCH_REPLY: &str = r#"{"candidates":[{"content":{"parts":[{"functionCall":{"name":"dispatch","args":{"drone_count":2,"place":"door"},"id":"call_4541055487298339433"}}],"role":"model"},"finishReason":"STOP","index":0}]}"#;
    // Hand-written (not a live capture): a reply with a real text part and no function call.
    const TEXT_ONLY_REPLY: &str = r#"{"candidates":[{"content":{"parts":[{"text":"That is not a command for the swarm."}],"role":"model"},"finishReason":"STOP","index":0}]}"#;
    // Live reply to "Aether, send drone four there": drone ids and the pointing flag.
    const THERE_REPLY: &str = r#"{"candidates":[{"content":{"parts":[{"functionCall":{"name":"dispatch","args":{"drone_ids":[4],"there":true},"id":"call_9540850045841806604"}}],"role":"model"},"finishReason":"STOP","index":0}]}"#;
    // Live reply to "Aether, bring drone two home and land it": both count and ids, which must be rejected.
    const BOTH_COUNT_AND_IDS_REPLY: &str = r#"{"candidates":[{"content":{"parts":[{"functionCall":{"name":"recall","args":{"drone_count":1,"drone_ids":[2]},"id":"call_2134192838450156528"}}],"role":"model"},"finishReason":"STOP","index":0}]}"#;
    // DISPATCH_REPLY with one extra argument added by hand (the model did not produce one on its own).
    const UNKNOWN_ARG_REPLY: &str = r#"{"candidates":[{"content":{"parts":[{"functionCall":{"name":"dispatch","args":{"drone_count":2,"place":"door","speed":5},"id":"call_4541055487298339433"}}],"role":"model"},"finishReason":"STOP","index":0}]}"#;

    fn reply(text: &str) -> Value {
        serde_json::from_str(text).unwrap()
    }

    #[test]
    fn dispatch_reply_maps_to_the_dispatch_command() {
        let command = to_command(&reply(DISPATCH_REPLY)).unwrap().unwrap();
        assert_eq!(command.to_json(), r#"{"kind":"dispatch","drones":{"count":2},"target":{"place":"door"}}"#);
    }

    #[test]
    fn there_reply_maps_to_the_there_target() {
        let command = to_command(&reply(THERE_REPLY)).unwrap().unwrap();
        assert_eq!(command.to_json(), r#"{"kind":"dispatch","drones":{"ids":[4]},"target":"there"}"#);
    }

    #[test]
    fn text_only_reply_gives_no_command() {
        assert_eq!(to_command(&reply(TEXT_ONLY_REPLY)), Ok(None));
    }

    #[test]
    fn unknown_argument_is_rejected() {
        assert!(to_command(&reply(UNKNOWN_ARG_REPLY)).unwrap_err().contains("speed"));
    }

    #[test]
    fn count_and_ids_together_are_rejected() {
        assert!(to_command(&reply(BOTH_COUNT_AND_IDS_REPLY)).is_err());
    }

    #[test]
    fn place_is_normalised() {
        let mut wire_reply = reply(DISPATCH_REPLY);
        wire_reply["candidates"][0]["content"]["parts"][0]["functionCall"]["args"]["place"] = json!("The Door ");
        let command = to_command(&wire_reply).unwrap().unwrap();
        assert_eq!(command.to_json(), r#"{"kind":"dispatch","drones":{"count":2},"target":{"place":"door"}}"#);
    }
}
