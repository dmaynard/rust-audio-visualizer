# Development Blog

## 2026-01-24: Project Kickoff

### Initial Request
The- Verified WASM build. a Rust Audio Visualizer using Median Cut for color quantization.
Hosting target: Netlify or GitHub Pages.

## 2026-01-24: Troubleshooting "Recursive Use" Error
- **Issue**: `wasm-bindgen` threw "recursive use of an object" when passing `Uint8Array` to Rust `&[u8]`.
- **Cause**: Likely due to aliasing safety checks when passing a View of WASM memory back into WASM, or `wasm-bindgen` internal borrowing.
- **Fix**: Changed `process_audio` signature to `Vec<u8>`.
- **Performance**: Negligible impact. Copying ~128 bytes per frame (FFT data) takes nanoseconds.

### Plan
- Initialize Vite + React + TypeScript project.
- Initialize Rust crate with `wasm-pack`.
- Use `median_cut` or `image` crate for quantization.
- Implement audio analysis and palette manipulation.

## 2026-01-24: Environment Check
Encountered Node.js v8.11.2. Requiring update to Node 18+ for Vite. Blocked on user update.

## 2026-01-24: Environment Check
Encountered Node.js v8.11.2. Requiring update to Node 18+ for Vite. Blocked on user update.

## 2026-02-03: HSL Palette Animation
- **Goal**: Improve color dynamics and prevent hue shifts.
- **Change**: Switched from RGB scaling to HSL interpolation.
- **Logic**: 
    - Converted palette to HSL on load.
    - Modulated `Lightness` and `Saturation` based on audio energy.
    - 1.0 Baseline: Silence (energy=0) now restores the exact original image colors.
- **Status**: Implemented in Rust core on `feature/hsl-palette-animation`. Verified build.

## 2026-02-09: Microphone Integration
- **Goal**: Allow real-time visualization of environmental audio.
- **Implementation**: Added `navigator.mediaDevices.getUserMedia` support to `Visualizer.tsx`.
- **Features**: 
    - Toggle button for Mic.
    - Automatic audio source switching (File <-> Mic).
    - Feedback prevention (Mic not connected to speakers).
    - **Device Selection**: Address macOS Continuity Camera issues.

## 2026-02-09: Frequency Response Tuning & Testing
- **Goal**: Fix "illusion" of coupled frequency response.
- **Fix**: 
    - **Steeper Mapping**: Changed frequency bin mapping curve from `pow(1.5)` to `pow(1.8)` to separate bass/mids better.
    - **Noise Gate**: Added `0.03` energy threshold to ignore broadband background noise.
- **Validation**: Generated `sweep_stereo.wav` (20Hz-20kHz Log Sweep) to visually confirm frequency separation.
- **Assets**: Configured `FlammarionColor.png` and Chopin audio to load automatically on startup.
- **Assets**: Configured `FlammarionColor.png` and Chopin audio to load automatically on startup.

## 2026-02-11: Equalizer Mode (Hybrid Rendering)
- **Feature**: Added a bar chart view to visualize the raw frequency data driving the image.
- **Architecture**:
    - **Rust**: Performs FFT and color modulation. Exposes `BIN_PEAKS` (f32) and `PALETTE` (u8) via pointers.
    - **JavaScript**: Reads memory directly (Zero-Copy) and renders bars using HTML5 Canvas.
- **Optimization**: Used `useRef` to fix stale closure bugs in the animation loop, ensuring instant view toggling.
