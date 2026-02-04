use wasm_bindgen::prelude::*;
use image::DynamicImage;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

// STATIC BUFFERS - Version 15 (The Nuclear Option)
// STATIC BUFFERS - Version 15 (The Nuclear Option)
const MAX_WIDTH: usize = 1600;
const MAX_HEIGHT: usize = 1200;
const MAX_PIXELS: usize = MAX_WIDTH * MAX_HEIGHT;
const BUFFER_SIZE: usize = MAX_PIXELS * 4;
const INPUT_SIZE: usize = 2048; // Ample space for FFT data

static mut DISPLAY_BUFFER: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];
static mut PIXELS: [u8; MAX_PIXELS] = [0; MAX_PIXELS];
static mut INPUT_BUFFER: [u8; INPUT_SIZE] = [0; INPUT_SIZE];
static mut RAW_IMAGE_BUFFER: [u8; MAX_PIXELS * 3] = [0; MAX_PIXELS * 3]; // RGB

// Fixed size arrays for Palette (Max 256 colors * 3 channels = 768)
static mut PALETTE: [u8; 768] = [0; 768];
static mut ORIGINAL_PALETTE: [u8; 768] = [0; 768];
static mut BIN_PEAKS: [f32; 256] = [0.1; 256];

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
        unsafe {
            // Reset logic
            IMG_WIDTH = 0;
            IMG_HEIGHT = 0;
            INPUT_LEN = 0;
            ACTIVE_PALETTE_LEN = 0;
        }
        AudioVisualizer
    }

    pub fn load_image(&self, width: u32, height: u32, data: &[u8]) -> Result<(), JsValue> {
        unsafe {
            if width as usize > MAX_WIDTH || height as usize > MAX_HEIGHT {
                 return Err(JsValue::from_str("Image too large for static buffer"));
            }
            IMG_WIDTH = width;
            IMG_HEIGHT = height;
        }

        unsafe {
            let pixel_count = (width * height) as usize;
            let expected_len = pixel_count * 4;
            if data.len() != expected_len {
                 return Err(JsValue::from_str("Data length mismatch"));
            }

            for i in 0..pixel_count {
                let src_idx = i * 4;
                let dst_idx = i * 3;
                RAW_IMAGE_BUFFER[dst_idx] = data[src_idx];
                RAW_IMAGE_BUFFER[dst_idx + 1] = data[src_idx + 1];
                RAW_IMAGE_BUFFER[dst_idx + 2] = data[src_idx + 2];
            }
        }
        
        self.set_color_count(64);
        
        Ok(())
    }

    pub fn set_color_count(&self, count: u8) {
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
            let (palette, _) = median_cut(&sample_pixels, count as usize);
            
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
                
                // Reset peaks
                BIN_PEAKS[i] = 0.1;
            }

            // 3. Map pixels
            // Create a temporary slice view for matching to avoid accessing global PALETTE repeatedly in loop overhead?
            // Actually, direct access is fast.
            
            // We can't iterate PALETTE easily because it's [u8; 768].
            // Let's make a local copy of colors for matching.
            let colors: Vec<[u8; 3]> = (0..p_len).map(|i| {
                [PALETTE[i*3], PALETTE[i*3+1], PALETTE[i*3+2]]
            }).collect();

            for i in 0..pixel_count {
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
            
            self.render();
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
        
            let palette_colors = ACTIVE_PALETTE_LEN / 3;
            if palette_colors == 0 { return 0.0; }
            
            // Quadratic Scaling (Pseudo-Log) to match human hearing
            // This grants more resolution to low frequencies (Bass) and compresses high frequencies
            let len_f = len as f32;
            let pc_f = palette_colors as f32;
            
            for i in 0..palette_colors {
                let i_f = i as f32;
                // Formula: bin = len * (i / count)^1.5 (Less aggressive than squared, more coverage)
                let start_ratio = (i_f / pc_f).powf(1.5);
                let end_ratio = ((i_f + 1.0) / pc_f).powf(1.5);
                
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
                
                let energy = max_val as f32 / 255.0;

                // AGC Implementation - Aggressive Tuning for Dynamics
                // Decay peak fast (drops 10% per frame)
                BIN_PEAKS[i] *= 0.90; 
                // Lower floor to 0.005 (1/200) to catch faint highs
                if BIN_PEAKS[i] < 0.005 { BIN_PEAKS[i] = 0.005; }
                
                // Pump peak up if current energy is higher
                if energy > BIN_PEAKS[i] {
                    BIN_PEAKS[i] = energy;
                }
                
                let normalized = energy / BIN_PEAKS[i];
                
                // Non-linear response (Square it) to emphasize beats
                // Range: 0.5 (quiet) to 1.8 (loud)
                let effect = 0.5 + (normalized * normalized * 1.3);

                let base_idx = i * 3;
                // Bounds check
                if base_idx + 2 < 768 {
                    let r_orig = ORIGINAL_PALETTE[base_idx] as f32;
                    let g_orig = ORIGINAL_PALETTE[base_idx+1] as f32;
                    let b_orig = ORIGINAL_PALETTE[base_idx+2] as f32;
                    
                    // Explicit saturation clamping to prevent wrap-around
                    let r_new = r_orig * effect;
                    let g_new = g_orig * effect;
                    let b_new = b_orig * effect;

                    PALETTE[base_idx] = if r_new > 255.0 { 255 } else { r_new as u8 };
                    PALETTE[base_idx+1] = if g_new > 255.0 { 255 } else { g_new as u8 };
                    PALETTE[base_idx+2] = if b_new > 255.0 { 255 } else { b_new as u8 };
                }
            }
            
            0.0 
        }
    }

    pub fn render(&self) {
        unsafe {
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
