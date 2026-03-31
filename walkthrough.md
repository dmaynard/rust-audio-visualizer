# HSL Palette Animation Walkthrough

I have implemented HSL-based palette animation to improve color dynamics and ensure silence restores the original image colors.

## Switch to HSL Space
Previously, we scaled RGB values directly:
```rust
// Old RGB scaling
let r_new = r_orig * effect;
```
This caused hue shifts (e.g., orange becoming yellow as it got brighter) and offered no independent control over saturation.

Now, we convert the palette to HSL once on image load:
```rust
// Store HSL
let (h, s, l) = rgb_to_hsl(r, g, b);
PALETTE_HSL[i] = ...
```

## Dynamics Logic
In `process_frequencies`, we modulate Lightness and Saturation separately:

```rust
// 1.0 Baseline for Silence (User Request)
let punch = normalized * normalized; // Non-linear response for beats

// Lightness: Up to 50% brighter
let l_mult = 1.0 + (punch * 0.5);

// Saturation: Up to 30% more saturated
let s_mult = 1.0 + (punch * 0.3);

let new_l = (l * l_mult).min(1.0);
let new_s = (s * s_mult).min(1.0);
```

This ensures that when `punch` (energy) is 0, the multiplier is `1.0`, restoring the exact original color.

## Verification
- **Build**: `cargo check` passed.
- **Logic**: Checked for boundaries (clamping S and L to 1.0) and baseline 1.0.

## Fix: Silence/Pause Behavior
Previously, pausing the audio left the last processed frame on screen, often with heightened saturation.
- **Change**: In `Visualizer.tsx`, `pauseAudio()` now explicitly sends a zero-filled buffer to WASM and processes one frame.
- **Result**: The image snaps back to its original "Zero Energy" palette immediately upon pausing.

## Fix: Exact Restoration (Darker Image Issue)
User reported that after pausing, the image was "considerably darker" than the original, despite the silence fix.
- **Cause**: HSL <-> RGB conversion involves floating point math and rounding, which likely introduced lossiness, making colors slightly different (and apparently darker) than the original `u8` bytes.
- **Fix**: In `lib.rs`, added a check for **Total Silence** (max energy == 0). If detected, it bypasses HSL calculations entirely and copies `ORIGINAL_PALETTE` (the pristine bytes from load time) directly to `PALETTE`.
- **Result**: The image is now guaranteed to be bit-exact to the original quantization when silent.

## Fix: "Statically Lighter" / Washout
User reported that colors were becoming "almost white" and "statically lighter" during playback.
- **Cause**: The AGC decay factor `0.90` was too aggressive, causing the visualizer to "forget" peaks instantly. The lightness modulation `l_mult = 1.0 + punch * 0.5` was pushing everything to white.
- **Fix**:
    1.  Slowed AGC Decay to `0.995` (stable floor).
    2.  Implemented "Headroom-Aware" Lightness boost: `l + (1.0 - l) * punch * 0.3`. (Only boosts what fits).
    3.  Reduced Saturation boost to `1.0 + punch * 0.4`.

## Fix: Crash on Large Image Load (Wasm Memory)
When loading a second, larger image, the app crashed with `RuntimeError: unreachable` and `recursive use of object`.
- **Cause**: Passing a large `Uint8Array` as an argument to a Wasm function causes `wasm-bindgen` to allocate memory in the Wasm heap to copy the data. If the heap grows or becomes fragmented/corrupted during this process (or if `free` fails), it traps.
- **Fix**: Implemented **Zero-Copy Upload**.
    1.  Added `static mut UPLOAD_BUFFER` (14MB) in Rust.
    2.  Exposed `get_upload_buffer_ptr()` to JS.
    3.  JS writes image bytes *directly* into Wasm memory (no allocation).
    4.  Rust reads from the static buffer.
    5.  Also fixed a race condition by pausing the animation loop during upload.

## Next Steps
- Run the visualizer in the browser.
- Verify that dark colors don't wash out (L modulation handles this, but black (L=0) stays black).

## Fix: Color Count Buttons (Quantization Logic Restoration)
Buttons for 16, 32, 64 colors stopped working after the Zero-Copy Refactor because the logic was accidentally commented out during debugging.
- **Cause**: The `median_cut` and `pixel_mapping` logic in `set_color_count` was disabled to isolate the crash.
- **Fix**: Restored the logic in `lib.rs` and added safety locking in `Visualizer.tsx` to prevent race conditions during rapid clicks.
- **Result**: Changing color count now instantly re-quantizes the image.

## Feature: Microphone Integration
Added a "Start Mic" button to visualize live audio input.
- **Implementation**: Uses `navigator.mediaDevices.getUserMedia` to create a `MediaStreamSourceNode`.
- **Switching**: Automatically pauses file playback when mic is activated, and stops mic when file playback is requested.
- **Feedback Loop Prevention**: The mic source is connected *only* to the `AnalyserNode`, not the `destination` (speakers), to prevent audio feedback.
- **Device Selection**: Added a dropdown to explicitly select the input device (fixing generic "Default" device issues on macOS/iOS).

## Test Audio
- Generated a **Stereo 20Hz-20kHz Logarithmic Sweep** (`public/sweep_stereo.wav`) to verify frequency response and channel mapping.
- This file allows confirming that:
    - Low frequencies (20Hz) map to the left/beginning of the palette.
    - High frequencies (20kHz) map to the right/end.
    - Stereo playback works correctly.

## Default Assets
- On startup, the app now automatically loads:
## Equalizer Mode
- **Hybrid Rendering**:
    - **Rust**: Calculates frequency energy (`BIN_PEAKS`) and modulated colors (`PALETTE`).
    - **JavaScript**: Reads raw data from WASM memory (Zero-Copy) and renders the bar chart on HTML5 Canvas.
- **Toggle**: Switch instantly between "Image" and "Chart" views using the UI button.
- **Synchronization**: Bars are colored exactly like the corresponding palette entry in the image.

## Fix: Low Frequency Resolution
- **Issue**: In 64-color mode, the first few frequency bands were identical due to low FFT resolution (`fftSize=256` -> 128 bins).
- **Fix**: Increased `fftSize` to `2048` (1024 bins).
- **Result**: Bass frequencies are now distinct and high-resolution, even with few color bands. Performance remains excellent (60fps).

## Feature: Maximize Screen Space
- **Goal**: Allow the visualizer to occupy the full screen.
- **Implementation**:
    - Removed fixed-width container and header.
    - Moved "Rust Audio Visualizer" title and description to a new **About Overlay**.
    - Added a floating ℹ️ button to toggle the overlay.
    - set `width: 100vw; height: 100vh` on the root container.
- **Result**: The visualizer canvas now dynamically scales to fill the available window space.
