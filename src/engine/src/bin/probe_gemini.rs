//! Sends one spoken command to Gemini 3 times and checks each reply maps to the same Command.

use std::process::ExitCode;
use std::time::Instant;

use aether_engine::gemini;

const TRANSCRIPT: &str = "Aether, send two drones to the door";

#[tokio::main]
async fn main() -> anyhow::Result<ExitCode> {
    dotenvy::dotenv().ok();
    let key = std::env::var("GEMINI_API_KEY")?;
    let model = std::env::var("GEMINI_MODEL")?;
    eprintln!("model={model}");

    match run(&key, &model).await {
        Ok((command, median_ms)) => {
            println!("gemini_ok command={command} calls=3 median_ms={median_ms}");
            Ok(ExitCode::SUCCESS)
        }
        Err(reason) => {
            println!("gemini_fail {reason}");
            Ok(ExitCode::from(1))
        }
    }
}

/// Returns the one Command all three calls agreed on, and the median latency in ms.
async fn run(key: &str, model: &str) -> Result<(String, u128), String> {
    let client = reqwest::Client::new();
    let mut commands = Vec::new();
    let mut latencies = Vec::new();
    for call in 1..=3 {
        let started = Instant::now();
        let reply = gemini::generate(&client, key, model, TRANSCRIPT)
            .await
            .map_err(|e| format!("call {call} request failed: {e}"))?;
        let ms = started.elapsed().as_millis();
        eprintln!("call {call} latency_ms={ms}");
        latencies.push(ms);
        let command = gemini::to_command(&reply).map_err(|e| format!("call {call} mapping failed: {e}"))?;
        let command = command.ok_or_else(|| format!("call {call} gave no Command"))?;
        commands.push(command.to_json());
    }
    if commands.iter().any(|c| *c != commands[0]) {
        return Err(format!("calls differed: {commands:?}"));
    }
    latencies.sort();
    Ok((commands.swap_remove(0), latencies[1]))
}
