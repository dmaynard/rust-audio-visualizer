#![allow(static_mut_refs)]
#![allow(unexpected_cfgs)]
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

// STATIC BUFFERS - Version 15 (The Nuclear Option)
// STATIC BUFFERS - Version 15 (The Nuclear Option)
const MAX_WIDTH: usize = 2560;
const MAX_HEIGHT: usize = 1440;
const MAX_PIXELS: usize = MAX_WIDTH * MAX_HEIGHT;
const BUFFER_SIZE: usize = MAX_PIXELS * 4;
const INPUT_SIZE: usize = 2048; // Ample space for FFT data
const UPLOAD_SIZE: usize = MAX_PIXELS * 4;

static mut DISPLAY_BUFFER: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];
static mut PIXELS: [u8; MAX_PIXELS] = [0; MAX_PIXELS];
static mut INPUT_BUFFER: [u8; INPUT_SIZE] = [0; INPUT_SIZE];
static mut RAW_IMAGE_BUFFER: [u8; MAX_PIXELS * 3] = [0; MAX_PIXELS * 3]; // RGB
static mut UPLOAD_BUFFER: [u8; UPLOAD_SIZE] = [0; UPLOAD_SIZE];

// Fixed size arrays for Palette (Max 256 colors * 3 channels = 768)
static mut PALETTE: [u8; 768] = [0; 768];
static mut ORIGINAL_PALETTE: [u8; 768] = [0; 768];
static mut PALETTE_HSL: [f32; 768] = [0.0; 768]; // H, S, L interleaved
static mut BIN_PEAKS: [f32; 256] = [0.1; 256];
static mut SPECTRUM: [f32; 256] = [0.0; 256];
static mut AGC_DECAY: f32 = 0.995;
static mut GLOBAL_GAIN: f32 = 1.0;

// State moved to Statics
static mut IMG_WIDTH: u32 = 0;
static mut IMG_HEIGHT: u32 = 0;
static mut INPUT_LEN: usize = 0;
static mut ACTIVE_PALETTE_LEN: usize = 0; // Number of bytes used in palette (colors * 3)

#[wasm_bindgen]
pub struct AudioVisualizer;

#[wasm_bindgen]
impl AudioVisualizer {
    #[wasm_bindgen(constructor)]
    pub fn new() -> AudioVisualizer {
        console_error_panic_hook::set_once();
        log("Rust Core: Initialized (v3 Tuned: Slow AGC + Headroom Fix)");
        unsafe {
            // Reset logic
            IMG_WIDTH = 0;
            IMG_HEIGHT = 0;
            INPUT_LEN = 0;
            ACTIVE_PALETTE_LEN = 0;
        }
        AudioVisualizer
    }

    pub fn get_upload_buffer_ptr(&self) -> *mut u8 {
        unsafe { UPLOAD_BUFFER.as_mut_ptr() }
    }

    pub fn load_image(&self, width: u32, height: u32) {
        unsafe {
            if width as usize > MAX_WIDTH || height as usize > MAX_HEIGHT {
                 // log("Rust Error: Image too large");
                 return;
            }
            // log(&format!("Rust: Loading Image {}x{}", width, height));
            IMG_WIDTH = width;
            IMG_HEIGHT = height;
        }
        
        unsafe {
            let pixel_count = (width * height) as usize;
            // No data len check needed (buffer is fixed/large enough by def, verified by MAX check)

            for i in 0..pixel_count {
                let src_idx = i * 4;
                let dst_idx = i * 3;
                // Read from UPLOAD_BUFFER instead of data slice
                RAW_IMAGE_BUFFER[dst_idx] = UPLOAD_BUFFER[src_idx];
                RAW_IMAGE_BUFFER[dst_idx + 1] = UPLOAD_BUFFER[src_idx + 1];
                RAW_IMAGE_BUFFER[dst_idx + 2] = UPLOAD_BUFFER[src_idx + 2];
            }
        
            // 2. Generate Palette (Delegated to set_color_count)
            // log("Rust: Stored Raw Image. Delegating to set_color_count.");
        }
        
        self.set_color_count(64);
        
        // log("Rust: load_image returning");
    }

    pub fn set_decay_factor(&self, factor: f32) {
        unsafe { AGC_DECAY = factor; }
    }

    pub fn set_global_gain(&self, gain: f32) {
        unsafe { GLOBAL_GAIN = gain; }
    }

    pub fn set_color_count(&self, count: u8) {
        // log("Rust: set_color_count start");
        unsafe {
            let pixel_count = (IMG_WIDTH * IMG_HEIGHT) as usize;
            if pixel_count == 0 { return; }
            
            // 1. Downsample
            let step = (pixel_count / 4096).max(1);
            let mut sample_pixels = Vec::with_capacity(4096);
            
            for i in (0..pixel_count).step_by(step) {
                let r = RAW_IMAGE_BUFFER[i*3];
                let g = RAW_IMAGE_BUFFER[i*3+1];
                let b = RAW_IMAGE_BUFFER[i*3+2];
                sample_pixels.push([r, g, b]);
            }

            // 2. Generate Palette
            // log("Rust: Generating Palette (Median Cut)");
            let (mut palette, _) = median_cut(&sample_pixels, count as usize);
            
            // Sort Palette by Hue
            palette.sort_by(|a, b| {
                let (h1, _, _) = rgb_to_hsl(a[0], a[1], a[2]);
                let (h2, _, _) = rgb_to_hsl(b[0], b[1], b[2]);
                h1.partial_cmp(&h2).unwrap_or(std::cmp::Ordering::Equal)
            });

            // log(&format!("Rust: Palette Generated ({} colors)", palette.len()));
            
            // Store to Static Arrays
            let p_len = palette.len().min(256);
            ACTIVE_PALETTE_LEN = p_len * 3;
            for (i, c) in palette.iter().take(p_len).enumerate() {
                PALETTE[i*3] = c[0];
                PALETTE[i*3+1] = c[1];
                PALETTE[i*3+2] = c[2];
                
                ORIGINAL_PALETTE[i*3] = c[0];
                ORIGINAL_PALETTE[i*3+1] = c[1];
                ORIGINAL_PALETTE[i*3+2] = c[2];

                let (h, s, l) = rgb_to_hsl(c[0], c[1], c[2]);
                PALETTE_HSL[i*3] = h;
                PALETTE_HSL[i*3+1] = s;
                PALETTE_HSL[i*3+2] = l;
            }

            // Clear BIN_PEAKS entirely to avoid stale data from previous palette size
            for i in 0..256 {
                BIN_PEAKS[i] = 0.01;
            }

            // 3. Map Pixels to Palette
            // Create a temporary slice view for matching to avoid accessing global PALETTE repeatedly in loop overhead?
            // Actually, direct access is fast.
            
            // We can't iterate PALETTE easily because it's [u8; 768].
            // Let's make a local copy of colors for matching.
            let colors: Vec<[u8; 3]> = (0..p_len).map(|i| {
                [PALETTE[i*3], PALETTE[i*3+1], PALETTE[i*3+2]]
            }).collect();

            // Check input array bounds before loop
            let max_idx_check = (pixel_count - 1) * 3 + 2;
            log(&format!("Rust: Starting Pixel Mapping. PixelCount={} MaxIdx={}", pixel_count, max_idx_check));

            for i in 0..pixel_count {
                if i % 500_000 == 0 { log(&format!("Rust: Mapping Pixel {}", i)); }

                let r = RAW_IMAGE_BUFFER[i*3];
                let g = RAW_IMAGE_BUFFER[i*3+1];
                let b = RAW_IMAGE_BUFFER[i*3+2];
                
                let mut min_dist = std::i32::MAX;
                let mut best_idx = 0;
                
                for (idx, color) in colors.iter().enumerate() {
                    let dr = r as i32 - color[0] as i32;
                    let dg = g as i32 - color[1] as i32;
                    let db = b as i32 - color[2] as i32;
                    let dist = dr*dr + dg*dg + db*db;
                    
                    if dist < min_dist {
                        min_dist = dist;
                        best_idx = idx;
                    }
                }
                PIXELS[i] = best_idx as u8;
            }
            log("Rust: Pixel Mapping Complete");
            
            self.render();
            log("Rust: Initial Render Complete");
        }
        
    }

    pub fn resize_input_buffer(&self, size: usize) {
        unsafe {
            if size <= INPUT_SIZE {
                INPUT_LEN = size;
            }
        }
    }
    
    pub fn get_input_buffer_ptr(&self) -> *const u8 {
        unsafe { INPUT_BUFFER.as_ptr() }
    }

    pub fn process_frequencies(&self) -> f32 {
        unsafe {
            let len = INPUT_LEN;
            if len == 0 { return 0.0; }
        
            let active_len = ACTIVE_PALETTE_LEN;
            let palette_colors = active_len / 3;
            if palette_colors == 0 { return 0.0; }
            
            
            // Quadratic Scaling (Pseudo-Log) to match human hearing
            // This grants more resolution to low frequencies (Bass) and compresses high frequencies
            let len_f = len as f32;
            
            // NEW: Scan for total silence first to ensure exact restoration (bypass HSL float errors)
            let mut max_energy_all: u8 = 0;
            for k in 0..len {
                 let val = INPUT_BUFFER[k];
                 if val > max_energy_all { max_energy_all = val; }
            }

            // Log energy for debugging
            // log(&format!("Max Energy: {}", max_energy_all));

            if max_energy_all == 0 {
                // log("Silence detected! Restoring ORIGINAL_PALETTE");
                // Restore exact original palette
                for j in 0..active_len {
                    PALETTE[j] = ORIGINAL_PALETTE[j];
                }
                return 0.0;
            }

            for i in 0..palette_colors {
                let i_f = i as f32;
                // Formula: bin = len * (i / count)^1.8 (Steeper curve to separate bass/mids more)
                let pc_f = palette_colors as f32;
                let start_ratio = (i_f / pc_f).powf(1.8);
                let end_ratio = ((i_f + 1.0) / pc_f).powf(1.8);
                
                let start_bin = (start_ratio * len_f) as usize;
                let end_bin = (end_ratio * len_f) as usize;
                
                // Ensure at least 1 bin width
                let end_bin = end_bin.max(start_bin + 1).min(len);
                
                let mut max_val: u8 = 0;
                
                for b in start_bin..end_bin {
                    if b >= INPUT_SIZE { break; } 
                    let val = INPUT_BUFFER[b];
                    if val > max_val { max_val = val; }
                }
                
                let mut energy = (max_val as f32 / 255.0) * GLOBAL_GAIN;
                if energy > 1.0 { energy = 1.0; }

                // Noise Gate: Cut low-level broadband noise which causes "all-on" look
                if energy < 0.03 { energy = 0.0; }

                // AGC Implementation
                // Slow down decay significantly to prevent "pumping" on low noise
                BIN_PEAKS[i] *= AGC_DECAY; 
                // Increase floor to 0.03 to match noise gate
                if BIN_PEAKS[i] < 0.03 { BIN_PEAKS[i] = 0.03; }
                if energy > BIN_PEAKS[i] { BIN_PEAKS[i] = energy; }
                
                let normalized = if BIN_PEAKS[i] > 0.0 { energy / BIN_PEAKS[i] } else { 0.0 };
                SPECTRUM[i] = energy;
                
                // HSL Modulation Logic
                // User Requirement: "Zero energy to be the same as the original palette"
                // So at normalized (or energy) ~ 0, we should have multipliers of 1.0.
                
                // We'll use the squared normalized energy to make it punchy on beats.
                let punch = normalized * normalized; 
                
                let base_idx = i * 3;
                if base_idx + 2 < 768 {
                    let h = PALETTE_HSL[base_idx];
                    let s_orig = PALETTE_HSL[base_idx+1];
                    let l_orig = PALETTE_HSL[base_idx+2];
                    
                    // Headroom-aware Lightness Boost
                    // prevents clipping whites by scaling boost based on remaining headroom
                    // Reduced boost factor 0.6 -> 0.3 to reduce washout
                    let headroom = 1.0 - l_orig;
                    let new_l = l_orig + (headroom * punch * 0.3);

                    // Saturation Boost
                    let new_s = (s_orig * (1.0 + punch * 0.4)).min(1.0);

                    // Debug output for first color only (Commented out for production)
                    // if i == 0 {
                    //    log(&format!("Bin 0: P={:.2} L_old={:.2} L_new={:.2}", punch, l_orig, new_l));
                    // }

                    let (r, g, b) = hsl_to_rgb(h, new_s, new_l);

                    PALETTE[base_idx] = r;
                    PALETTE[base_idx+1] = g;
                    PALETTE[base_idx+2] = b;
                }
            }
            
            0.0 
        }
    }

    pub fn render(&self) {
        unsafe {
            // log("Rust: render() start");
            let pixel_count = (IMG_WIDTH * IMG_HEIGHT) as usize;
            
            for i in 0..pixel_count {
                let color_idx = PIXELS[i] as usize;
                let base = i * 4;
                
                let p_idx = color_idx * 3;
                // Use fixed bound 768. Logic relies on valid color_idx from set_color_count
                if p_idx + 2 < 768 {
                    DISPLAY_BUFFER[base] = PALETTE[p_idx];
                    DISPLAY_BUFFER[base + 1] = PALETTE[p_idx + 1];
                    DISPLAY_BUFFER[base + 2] = PALETTE[p_idx + 2];
                    DISPLAY_BUFFER[base + 3] = 255;
                }
            }
        }
    }

    pub fn get_display_buffer_ptr(&self) -> *const u8 {
        unsafe { DISPLAY_BUFFER.as_ptr() }
    }

    pub fn get_display_buffer_len(&self) -> usize {
        unsafe { (IMG_WIDTH * IMG_HEIGHT * 4) as usize }
    }
    
    pub fn get_width(&self) -> u32 {
        unsafe { IMG_WIDTH }
    }
    
    pub fn get_height(&self) -> u32 {
        unsafe { IMG_HEIGHT }
    }

    pub fn get_spectrum_ptr(&self) -> *const f32 {
        unsafe { SPECTRUM.as_ptr() }
    }

    pub fn get_palette_ptr(&self) -> *const u8 {
        unsafe { PALETTE.as_ptr() }
    }
}

// Median Cut Implementation
#[derive(Clone, Copy, Debug)]
struct Pixel {
    r: u8,
    g: u8,
    b: u8,
    original_idx: usize,
}

struct Bucket {
    pixels: Vec<Pixel>,
}

impl Bucket {
    fn range(&self) -> (u8, u8, u8) {
        let mut min_r = 255; let mut max_r = 0;
        let mut min_g = 255; let mut max_g = 0;
        let mut min_b = 255; let mut max_b = 0;
        
        for p in &self.pixels {
            if p.r < min_r { min_r = p.r; }
            if p.r > max_r { max_r = p.r; }
            if p.g < min_g { min_g = p.g; }
            if p.g > max_g { max_g = p.g; }
            if p.b < min_b { min_b = p.b; }
            if p.b > max_b { max_b = p.b; }
        }
        (max_r - min_r, max_g - min_g, max_b - min_b)
    }
    
    fn volume(&self) -> u32 {
        let (r, g, b) = self.range();
        (r as u32) * (g as u32) * (b as u32)
    }

    fn split(mut self) -> (Bucket, Bucket) {
        let (r_range, g_range, b_range) = self.range();
        
        if r_range >= g_range && r_range >= b_range {
            self.pixels.sort_by(|a, b| a.r.cmp(&b.r));
        } else if g_range >= r_range && g_range >= b_range {
            self.pixels.sort_by(|a, b| a.g.cmp(&b.g));
        } else {
            self.pixels.sort_by(|a, b| a.b.cmp(&b.b));
        }
        
        let mid = self.pixels.len() / 2;
        let right_pixels = self.pixels.split_off(mid);
        
        (self, Bucket { pixels: right_pixels })
    }
    
    fn average_color(&self) -> [u8; 3] {
        if self.pixels.is_empty() { return [0, 0, 0]; }
        let mut sum_r: u64 = 0;
        let mut sum_g: u64 = 0;
        let mut sum_b: u64 = 0;
        
        for p in &self.pixels {
            sum_r += p.r as u64;
            sum_g += p.g as u64;
            sum_b += p.b as u64;
        }
        let count = self.pixels.len() as u64;
        [
            (sum_r / count) as u8,
            (sum_g / count) as u8,
            (sum_b / count) as u8,
        ]
    }
}

fn median_cut(pixels: &[[u8; 3]], depth: usize) -> (Vec<[u8; 3]>, Vec<u8>) {
    let mapped_pixels: Vec<Pixel> = pixels.iter().enumerate().map(|(i, &p)| {
        Pixel { r: p[0], g: p[1], b: p[2], original_idx: i }
    }).collect();

    let initial_bucket = Bucket { pixels: mapped_pixels };
    let mut buckets = vec![initial_bucket];
    
    // We want a fixed number of colors, e.g., 64.
    // The 'depth' arg passed is actually target count (64).
    let target_count = depth; 

    while buckets.len() < target_count {
        // Find bucket with largest range (or volume?) to split.
        // Usually splitting the one with largest range along longest axis is good.
        // Or volume? Volume is robust.
        
        // Find index of bucket to split
        let mut max_vol = 0;
        let mut split_idx = None;
        
        for (i, bucket) in buckets.iter().enumerate() {
            if bucket.pixels.len() > 1 {
                let vol = bucket.volume();
                // Simple tie-breaking or just >
                if vol >= max_vol {
                    max_vol = vol;
                    split_idx = Some(i);
                }
            }
        }
        
        match split_idx {
            Some(idx) => {
                let bucket = buckets.remove(idx);
                let (b1, b2) = bucket.split();
                buckets.push(b1);
                buckets.push(b2);
            },
            None => break, // Cannot split further
        }
    }
    
    let mut palette: Vec<[u8; 3]> = Vec::with_capacity(buckets.len());
    let mut indices: Vec<u8> = vec![0; pixels.len()];
    
    for (i, bucket) in buckets.iter().enumerate() {
        let color = bucket.average_color();
        palette.push(color);
        
        for p in &bucket.pixels {
            indices[p.original_idx] = i as u8;
        }
    }
    
    (palette, indices)
}

// HSL Helper Functions
fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let l = (max + min) / 2.0;
    let mut h = 0.0;
    let mut s = 0.0;

    if delta != 0.0 {
        s = if l > 0.5 { delta / (2.0 - max - min) } else { delta / (max + min) };

        if max == r {
            h = (g - b) / delta + (if g < b { 6.0 } else { 0.0 });
        } else if max == g {
            h = (b - r) / delta + 2.0;
        } else {
            h = (r - g) / delta + 4.0;
        }
        h /= 6.0;
    }

    (h, s, l)
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let r;
    let g;
    let b;

    if s == 0.0 {
        r = l;
        g = l;
        b = l;
    } else {
        let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
        let p = 2.0 * l - q;
        r = hue_to_rgb(p, q, h + 1.0 / 3.0);
        g = hue_to_rgb(p, q, h);
        b = hue_to_rgb(p, q, h - 1.0 / 3.0);
    }

    (
        (r * 255.0).round() as u8,
        (g * 255.0).round() as u8,
        (b * 255.0).round() as u8,
    )
}

fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
    if t < 0.0 { t += 1.0; }
    if t > 1.0 { t -= 1.0; }
    if t < 1.0 / 6.0 { return p + (q - p) * 6.0 * t; }
    if t < 1.0 / 2.0 { return q; }
    if t < 2.0 / 3.0 { return p + (q - p) * (2.0 / 3.0 - t) * 6.0; }
    p
}
