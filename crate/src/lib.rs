use wasm_bindgen::prelude::*;
use image::DynamicImage;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

// STATIC BUFFERS - Version 15 (The Nuclear Option)
const MAX_WIDTH: usize = 800;
const MAX_HEIGHT: usize = 600;
const MAX_PIXELS: usize = MAX_WIDTH * MAX_HEIGHT;
const BUFFER_SIZE: usize = MAX_PIXELS * 4;
const INPUT_SIZE: usize = 2048; // Ample space for FFT data

static mut DISPLAY_BUFFER: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];
static mut PIXELS: [u8; MAX_PIXELS] = [0; MAX_PIXELS];
static mut INPUT_BUFFER: [u8; INPUT_SIZE] = [0; INPUT_SIZE];
static mut PALETTE: Vec<u8> = Vec::new(); 
static mut ORIGINAL_PALETTE: Vec<u8> = Vec::new();

#[wasm_bindgen]
pub struct AudioVisualizer {
    width: u32,
    height: u32,
    // input_buffer_len tracked by JS side essentially, but we can store used len
    input_len: usize,
}

#[wasm_bindgen]
impl AudioVisualizer {
    #[wasm_bindgen(constructor)]
    pub fn new() -> AudioVisualizer {
        console_error_panic_hook::set_once();
        // Initialize palette vectors once
        unsafe {
            if PALETTE.capacity() == 0 {
                PALETTE = Vec::with_capacity(256);
                ORIGINAL_PALETTE = Vec::with_capacity(256);
            }
        }
        AudioVisualizer {
            width: 0,
            height: 0,
            input_len: 0,
        }
    }

    pub fn load_image(&mut self, data: &[u8]) -> Result<(), JsValue> {
        let img = image::load_from_memory(data)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        
        let img = img.resize(800, 600, image::imageops::FilterType::Lanczos3);
        self.width = img.width();
        self.height = img.height();
        
        // Safety check
        if self.width as usize > MAX_WIDTH || self.height as usize > MAX_HEIGHT {
             return Err(JsValue::from_str("Image too large for static buffer"));
        }

        // Convert to RGB8
        let rgb_img = img.to_rgb8();
        let raw_pixels = rgb_img.as_raw();
        let pixels: Vec<[u8; 3]> = raw_pixels.chunks(3).map(|c| [c[0], c[1], c[2]]).collect();

        // Quantize to 64 colors
        let (palette, indices) = median_cut(&pixels, 64);
        
        unsafe {
            PALETTE = palette.iter().flat_map(|c| c.to_vec()).collect();
            ORIGINAL_PALETTE = PALETTE.clone();
            
            // Copy indices to static PIXELS
            let len = indices.len().min(MAX_PIXELS);
            PIXELS[0..len].copy_from_slice(&indices[0..len]);
        }
        
        // Initial render
        self.render();
        
        Ok(())
    }

    pub fn resize_input_buffer(&mut self, size: usize) {
        // Just track the size, don't realloc static
        if size <= INPUT_SIZE {
            self.input_len = size;
        }
    }
    
    pub fn get_input_buffer_ptr(&self) -> *const u8 {
        unsafe { INPUT_BUFFER.as_ptr() }
    }

    pub fn process_frequencies(&mut self) -> f32 {
        let len = self.input_len;
        if len == 0 { return 0.0; }
        
        unsafe {
            if PALETTE.is_empty() { return 0.0; }
            
            let bass_end = len / 3;
            
            // Simple safe sum
            let mut bass: u32 = 0;
            for i in 0..bass_end {
                 bass += INPUT_BUFFER[i] as u32;
            }
            
            let bass_count = (bass_end as u32).max(1);
            let bass_avg = (bass / bass_count) as f32 / 255.0;
            
            // Animation: Mutate Palette
            let effect = 0.5 + 4.0 * bass_avg;
            
            for (i, pixel) in PALETTE.chunks_exact_mut(3).enumerate() {
                let base_idx = i * 3;
                if base_idx + 2 >= ORIGINAL_PALETTE.len() { break; }
                
                let r_orig = ORIGINAL_PALETTE[base_idx] as f32;
                let g_orig = ORIGINAL_PALETTE[base_idx+1] as f32;
                let b_orig = ORIGINAL_PALETTE[base_idx+2] as f32;
                
                pixel[0] = (r_orig * effect).min(255.0) as u8;
                pixel[1] = (g_orig * effect).min(255.0) as u8;
                pixel[2] = (b_orig * effect).min(255.0) as u8;
            }
            
            return bass_avg;
        }
    }

    pub fn render(&mut self) {
        let pixel_count = (self.width * self.height) as usize;
        let buffer_len = pixel_count * 4;
        
        unsafe {
            for i in 0..pixel_count {
                let color_idx = PIXELS[i] as usize;
                let base = i * 4;
                
                let p_idx = color_idx * 3;
                if p_idx + 2 < PALETTE.len() {
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
        (self.width * self.height * 4) as usize
    }
    
    pub fn get_width(&self) -> u32 {
        self.width
    }
    
    pub fn get_height(&self) -> u32 {
        self.height
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
