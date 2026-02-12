# Implementation Plan - HSL Palette Animation

## Goal
Improve the visualizer's color dynamics by processing palette animation in HSL (Hue, Saturation, Lightness) space instead of RGB. This prevents color shifting artifacts when scaling brightness and allows for more natural saturation boosting on beats.

## User Review Required
> [!NOTE]
> I will be implementing manual `rgb_to_hsl` and `hsl_to_rgb` conversion functions to keep the WASM binary size small, rather than adding a new dependency.

## Proposed Changes

### [Rust Core] `crate/src/lib.rs`

#### [MODIFY] Structs and Statics
- Add `static mut PALETTE_HSL: [f32; 768] = [0.0; 768];` (Stores H, S, L for each of the 256 max colors).
- Populate `PALETTE_HSL` in `set_color_count` (after median cut generates RGB palette).

#### [NEW] Helper Functions
- `fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f32, f32, f32)`
- `fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8)`

#### [MODIFY] [lib.rs](file:///Users/dmaynard/projects/rust-audio-visualizer/crate/src/lib.rs)
- Remove `data: &[u8]` argument from `load_image`.
- Add `static mut UPLOAD_BUFFER: [u8; MAX_PIXELS * 4]`.
- Add `get_upload_buffer_ptr()`.
- Update `load_image` to read from `UPLOAD_BUFFER`.

#### [MODIFY] [Visualizer.tsx](file:///Users/dmaynard/projects/rust-audio-visualizer/src/components/Visualizer.tsx)
- Use `wasmMemory` and `get_upload_buffer_ptr` to upload image data.
- Call `load_image(width, height)` (no data arg).

#### [MODIFY] `process_frequencies`
- Instead of scaling RGB directly (which can desaturate or shift hue):
    1. Retrieve original (H, S, L) from `PALETTE_HSL`.
    2. Calculate `energy` from audio (existing logic).
    3. Modulate `Lightness`: `L_new = L_orig * effect`.
        - Ensure dark colors can still light up (maybe add a base floor).
    4. Modulate `Saturation`: `S_new = S_orig * (1.0 + energy * 0.2)` (Boost saturation on loud beats).
    5. Convert back to RGB using `hsl_to_rgb`.
    6. Update `PALETTE`.

## Verification Plan

### Manual Verification
- **Visual Check**:
    - Play audio.
    - Observe if colors maintain their "identity" (Hue) while pulsing.
    - Check if "black" or very dark colors pulse correctly (might need special handling since 0 * effect = 0).
- **Performance**:
    - Ensure the extra math (float conversions) doesn't drop FPS below 60.

### Automated Tests
- None planned for this visual effect, as it relies on subjective "look and feel".

## Equalizer Mode Implementation
- **Goal**: Add a bar chart view driven by the same data as the image.
- **Rust (`lib.rs`)**:
    - [NEW] `get_spectrum_ptr()`: Expose `BIN_PEAKS` (f32 values 0.0-1.0).
    - [NEW] `get_palette_ptr()`: Expose modulated `PALETTE` (u8 RGB triplets).
- **TypeScript (`Visualizer.tsx`)**:
    - [NEW] `viewMode` state ('image' | 'equalizer').
    - [NEW] `drawEqualizer()`: Loop through palette bins, draw bar with height `spectrum[i] * H` and color `palette[i]`.
    - [UI] Toggle Button: "Chart" vs "Image".
