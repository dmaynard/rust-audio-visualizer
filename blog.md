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
