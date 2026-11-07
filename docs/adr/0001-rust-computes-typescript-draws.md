# Rust computes, TypeScript draws

Every calculation lives in Rust. A shared `aether-core` crate is compiled natively into `aether-engine` on the laptop and to WebAssembly as `aether-wasm` in the browser, so the same code gives the same numbers in both places. The voice loop (mic capture, speech-to-text streaming, wake rule, Gemini, policy check, spoken replies) also runs natively in the engine, so one process owns both the mic and the speaker. The browser's TypeScript only lays out the interface, draws with Three.js, and plays positional sound.

## Considered Options

- **All-Rust UI (Leptos or Dioxus):** rejected. Design iteration is slower without instant hot reload, and Three.js and the browser audio stack would each need glue code, with no gain in looks or speed.
- **Python or TypeScript backend with Rust only for hot paths:** rejected. Two backend languages means two runtimes to debug at 3am, and the brief is Rust wherever it does not cost usability or looks.
