# Implementation Plan - Rust Audio Visualizer

## Goal
Build a web application that visualizes audio by animating an image's color palette based on real-time audio power spectrum analysis. The core processing (FFT, image quantization, rendering) will use Rust/WASM for performance, sharing pixel memory directly with the JS canvas.

## User Review Required
> [!NOTE]
> - **Color Quantization**: User confirmed **Median Cut**.
> - **Hosting**: Validating Netlify or GitHub Pages.
> - **Documentation**: Maintain a `blog.md` development log.

## Proposed Architecture

### [Tech Stack]
- **Frontend**: Vite + React + TypeScript
- **Backend/Core**: Rust + `wasm-bindgen`
- **Build**: `vite-plugin-rsw` or `vite-plugin-wasm` with `wasm-pack`

### [Data Flow]
1. **Setup**:
    - User uploads Image -> JS reads data -> Passes to Rust.
    - Rust performs **Color Quantization** (reducing to 8/16/32/64 colors).
    - Rust creates an **Index Map** (pixels -> palette index).
    - Rust allocates a **Display Buffer** (RGBA) accessible by JS.
2. **Runtime Loop (per frame)**:
    - User uploads Audio -> JS `AudioContext` -> API.
    - JS gets `ByteFrequencyData` or raw samples -> Passes to Rust.
    - Rust computes **Power Spectrum** (if not done by JS AnalyzerNode) or processes the spectrum.
    - Rust **Modifies Palette** colors based on audio frequencies.
    - Rust **Reconstructs Image**: Iterates Index Map, looks up new Palette colors, writes to Display Buffer.
    - JS constructs `ImageData` from the WASM memory view and puts it on `Canvas`.

## Proposed Changes

### [Rust Component]
#### [DONE] `crate/src/lib.rs`
- **Struct `AudioVisualizer`**:
    - `width`, `height`: u32
    - `index_map`: `Vec<u8>` (stores palette index per pixel)
    - `palette`: `Vec<u8>` (current RGB palette)
    - `display_buffer`: `Vec<u8>` (flat RGBA buffer for Canvas)
- **Methods**:
    - `new()`
    - `load_image(data: &[u8])`: Decodes and quantizes image.
    - `process_audio(samples: &[u8])`: Updates palette based on audio frequency data.
    - `render()`: Updates `display_buffer`.

### [Frontend Component]
#### [NEW] `src/components/Visualizer.tsx`
- Handles `requestAnimationFrame`.
- Manages `AudioContext` and `AnalyserNode`.
- Bridges `wasm` memory to `canvas`.

## Verification Plan

### Manual Verification
- **Load Test**: Load large images and check FPS.
- **Audio Test**: Ensure audio plays and visualizer reacts synchronously.
- **Palette Test**: Verify switching between 8, 16, 32, 64 colors works correctly.
- **Memory Test**: Ensure to WASM memory leaks (cleanup if visualizer is reset).
