# Rust Audio Visualizer

**Authored by David S. Maynard (with AntiGravity assist)**

A high-performance, browser-based audio visualizer built with **React** and a custom **Rust WebAssembly (WASM)** core. This application dynamically extracts dominant color palettes from any uploaded image and pulses those colors in real-time to the frequency bands of an audio track.

---

## Basic Functions

- **Custom Media Uploads**: Drag and drop your own `.mp3`/`.wav` files and `.png`/`.jpg`/`.heic` images directly onto the canvas, or use the built-in UI buttons to upload files from mobile devices.
- **Median Cut Quantization**: The WASM core mathematically analyzes the uploaded image to extract the top distinct colors (up to 64) that represent the image's palette.
- **Dual Visualizer Modes**:
  - **🖼️ Image Mode**: The visualizer modulates the Lightness and Saturation (HSL) of the image's extracted color palette in real-time based on the energy of the audio frequencies.
  - **📊 Equalizer Mode**: A spectral frequency chart that visualizes the raw instantaneous audio energy, with bars perfectly matched to the image's color palette.
- **Real-Time Controls**: Adjust the number of colors extracted, tweak the WebAudio smoothing time constant, and boost quiet audio tracks using the global Gain slider.
- **Microphone Support**: Visualize live audio from your microphone instead of a static file.

---

## Basic Architecture

The project bridges the gap between browser APIs and systems-level performance by splitting responsibilities across two layers:

### 1. The Frontend (React + TypeScript + Vite)
The React application acts as the conductor. It handles all of the UI state, user interactions, and DOM rendering. 
- It utilizes the **WebAudio API** to create an `AnalyserNode`, which performs Fast Fourier Transforms (FFT) on the audio source to extract real-time frequency data (bins).
- It manages the **HTML5 `<canvas>`** element, pushing the final rendered image or equalizer shapes to the screen using `requestAnimationFrame`.

### 2. The Core Engine (Rust + WebAssembly)
To achieve 60 frames-per-second performance without lagging the browser, the heavy mathematical lifting is done in Rust and compiled to `.wasm`.
- **Zero-Copy Memory**: Instead of passing massive arrays of pixel data back and forth between JavaScript and Rust (which would cause massive garbage collection spikes), the Rust core allocates a static chunk of memory. JavaScript writes the audio frequencies directly into Rust's memory, and Rust writes the modified image pixels directly into a shared buffer that the `<canvas>` paints from.
- **Color Quantization**: The Rust core implements a custom **Median Cut algorithm** to efficiently scan over 1 million pixels and cluster them into a deterministic color palette.
- **Hue Sorting & HSL Modulation**: The color palette is sorted by Hue. When audio frequencies hit, Rust calculates the energy, applies an Automatic Gain Control (AGC) decay envelope, and modulates the HSL values of the pixels before pushing them to the shared display buffer.

---

## Running Locally

1. Install dependencies: `npm install`
2. Build the WASM core: `npm run build:wasm` *(Requires the Rust toolchain and `wasm-pack`)*
3. Start the dev server: `npm run dev`

---

## License

**MIT License**

Copyright (c) 2026 David S. Maynard

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
