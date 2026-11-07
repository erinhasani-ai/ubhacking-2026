# Hand landmarks come from MediaPipe in the browser

The laptop webcam's hand landmarks (21 points per hand, with handedness) come from Google's MediaPipe Hand Landmarker, running in the browser on WebAssembly and the GPU. Everything that reads those points is Rust in `aether-core`, run in the browser through `aether-wasm`: palm or back of the hand, hand shape, Push and Pull, Halt, and the cursor filter. ADR 0001 still holds for all code we write; the one exception is a trained model we did not write. Camera frames never leave the browser: only Gestures and the pointing ray go to the engine.

## Considered Options

- **The hand model run natively in Rust** (camera capture through a Rust crate, the model through an ONNX or TFLite runtime crate): rejected. Two unproven native pieces on this Intel Mac, camera permission for a terminal process and a model runtime, for no difference a judge can see, on top of an already slow cold build.
- **Landmarks in the browser, Gesture logic in the engine** (landmarks sent over the WebSocket): rejected. The cursor would wait for a round trip and the 30 Hz stream; run in the browser, it follows the finger on every video frame.
- **Hand tracking on the iPhone's camera**: rejected. The phone is already the Sensor and the Pointer, and a camera page on iOS needs HTTPS.

Unverified until T28: the landmark rate on this laptop while the Room Twin renders, and how reliably the reported handedness holds when the back of the hand faces the camera.
