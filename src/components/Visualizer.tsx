import React, { useEffect, useRef, useState } from 'react';
import init, { AudioVisualizer } from '../../crate/pkg/audio_visualizer_core';

export const Visualizer: React.FC = () => {
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const [visualizer, setVisualizer] = useState<AudioVisualizer | null>(null);
    const [wasmMemory, setWasmMemory] = useState<WebAssembly.Memory | null>(null);
    const sourceRef = useRef<AudioBufferSourceNode | null>(null);
    const audioBufferRef = useRef<AudioBuffer | null>(null);

    // Playback state
    const [isPlaying, setIsPlaying] = useState(false);
    const [startTime, setStartTime] = useState(0);
    const [pausedAt, setPausedAt] = useState(0);
    const [hasAudio, setHasAudio] = useState(false);
    const [colorCount, setColorCount] = useState(64); // Default 64

    const audioContextRef = useRef<AudioContext | null>(null);
    const analyserRef = useRef<AnalyserNode | null>(null);
    const animationFrameRef = useRef<number>(0);

    // Ref to track playing state inside requestAnimationFrame loop without stale closure
    const isPlayingRef = useRef(false);

    useEffect(() => {
        isPlayingRef.current = isPlaying;
    }, [isPlaying]);

    const handleColorCountChange = (count: number) => {
        setColorCount(count);
        if (visualizer) {
            visualizer.set_color_count(count);
            // Force a re-render of the frame to show new palette immediately
            // But only if we have an image loaded (width > 0)
            if (visualizer.get_width() > 0) {
                renderFrame();
            }
        }
    };

    useEffect(() => {
        const loadWasm = async () => {
            const module = await init();
            setWasmMemory(module.memory);
            const viz = new AudioVisualizer();
            setVisualizer(viz);
        };
        loadWasm();

        return () => {
            if (animationFrameRef.current) cancelAnimationFrame(animationFrameRef.current);
            stopAudio();
            audioContextRef.current?.close();
        };
    }, []);

    const stopAudio = () => {
        if (sourceRef.current) {
            sourceRef.current.stop();
            sourceRef.current.disconnect();
            sourceRef.current = null;
        }
    };

    const playAudio = () => {
        if (!audioContextRef.current || !audioBufferRef.current) return;

        // Ensure context is running
        if (audioContextRef.current.state === 'suspended') {
            audioContextRef.current.resume();
        }

        stopAudio(); // Stop any existing source
        if (animationFrameRef.current) cancelAnimationFrame(animationFrameRef.current);

        const ctx = audioContextRef.current;
        const source = ctx.createBufferSource();
        source.buffer = audioBufferRef.current;
        source.connect(ctx.destination);

        // Re-connect analyser
        if (!analyserRef.current) {
            const analyser = ctx.createAnalyser();
            analyser.fftSize = 256;
            analyserRef.current = analyser;
        }
        source.connect(analyserRef.current);

        // Setup initial buffer size once to avoid reallocation in loop
        if (visualizer) {
            visualizer.resize_input_buffer(analyserRef.current.frequencyBinCount);
        }

        // Calculate offset
        const offset = pausedAt;
        source.start(0, offset);

        setStartTime(ctx.currentTime - offset);
        setIsPlaying(true);
        isPlayingRef.current = true; // Sync update for animation loop
        sourceRef.current = source;

        source.onended = () => {
            // Optional: Handle natural end
        };

        animate();
    };

    const pauseAudio = () => {
        if (!audioContextRef.current || !sourceRef.current) return;

        const elapsed = audioContextRef.current.currentTime - startTime;
        setPausedAt(elapsed);

        stopAudio();
        setIsPlaying(false);
        if (animationFrameRef.current) cancelAnimationFrame(animationFrameRef.current);
    };

    const rewindAudio = () => {
        setPausedAt(0);
        setStartTime(0);
        if (isPlaying) {
            playAudio(); // Restart from 0
        } else {
            // Just reset state
            // If we were paused, we stay paused but at 0
        }
    };

    const handleImageUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
        if (!e.target.files || !e.target.files[0] || !visualizer) return;
        const file = e.target.files[0];

        // Use Image API to load and resize in JS (avoids WASM OOM)
        const img = new Image();
        const url = URL.createObjectURL(file);

        img.onload = () => {
            URL.revokeObjectURL(url);

            // Calculate dimensions satisfying max 1600x1200
            const MAX_W = 1600;
            const MAX_H = 1200;
            let w = img.width;
            let h = img.height;

            // Lanczos-like scaling logic (simplified aspect ratio preservation)
            const scale = Math.min(1.0, Math.min(MAX_W / w, MAX_H / h));
            w = Math.floor(w * scale);
            h = Math.floor(h * scale);

            // Helper Canvas
            const offscreen = document.createElement('canvas');
            offscreen.width = w;
            offscreen.height = h;
            const ctx = offscreen.getContext('2d');
            if (!ctx) return;

            // High quality resize
            ctx.imageSmoothingEnabled = true;
            ctx.imageSmoothingQuality = 'high';
            ctx.drawImage(img, 0, 0, w, h);

            const imageData = ctx.getImageData(0, 0, w, h);
            const data = new Uint8Array(imageData.data.buffer);

            try {
                visualizer.load_image(w, h, data);
                resizeCanvas();
                renderFrame();
            } catch (err) {
                console.error("Error loading image:", err);
                alert("Error loading image. Check console.");
            }
        };
        img.onerror = () => {
            URL.revokeObjectURL(url);
            alert("Failed to load image");
        };
        img.src = url;
    };

    const handleAudioUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
        if (!e.target.files || !e.target.files[0]) return;
        const file = e.target.files[0];
        const buffer = await file.arrayBuffer();

        if (!audioContextRef.current) {
            audioContextRef.current = new AudioContext();
        }

        const ctx = audioContextRef.current;
        const audioBuffer = await ctx.decodeAudioData(buffer);
        audioBufferRef.current = audioBuffer;

        setHasAudio(true);
        setPausedAt(0);
        setStartTime(0);
        playAudio();
    };

    const resizeCanvas = () => {
        if (!visualizer || !canvasRef.current) return;
        canvasRef.current.width = visualizer.get_width();
        canvasRef.current.height = visualizer.get_height();
    };

    const renderFrame = () => {
        if (!visualizer || !canvasRef.current || !wasmMemory) return;
        // visualizer.render() is called inside animate for audio, or manually for static image

        const width = visualizer.get_width();
        const height = visualizer.get_height();
        if (width === 0 || height === 0) return;

        const bufferPtr = visualizer.get_display_buffer_ptr();
        const len = visualizer.get_display_buffer_len();

        const memBuffer = new Uint8Array(wasmMemory.buffer);
        const imageBuffer = new Uint8ClampedArray(memBuffer.subarray(bufferPtr, bufferPtr + len));
        const imageData = new ImageData(imageBuffer, width, height);

        const ctx = canvasRef.current.getContext('2d');
        if (ctx) {
            ctx.putImageData(imageData, 0, 0);
        }
    };

    const animate = () => {
        if (!analyserRef.current || !visualizer || !wasmMemory) return;

        // Stop loop if not playing (checked via Ref to avoid stale closure)
        if (!isPlayingRef.current) return;

        const bufferLength = analyserRef.current.frequencyBinCount;

        // 1. Ensure Rust buffer is correct size (MOVED TO SETUP)
        // visualizer.resize_input_buffer(bufferLength);

        // 2. Get pointer to Rust buffer
        const inputPtr = visualizer.get_input_buffer_ptr();

        // 3. Create a temporary JS buffer to read audio data (Safe Double Buffering)
        const tempArray = new Uint8Array(bufferLength);
        analyserRef.current.getByteFrequencyData(tempArray);

        // 4. Copy into WASM memory
        // Create a view into WASM memory at the input pointer
        const wasmInputArray = new Uint8Array(wasmMemory.buffer, inputPtr, bufferLength);
        wasmInputArray.set(tempArray);

        // 5. Measure Rust Execution Time
        const t0 = performance.now();
        try {
            visualizer.process_frequencies();
            visualizer.render();
        } catch (e) {
            console.error("Rust execution error:", e);
        }
        const t1 = performance.now();

        // 6. Measure Render/Paint Time
        renderFrame();
        const t2 = performance.now();

        // Log performance stats every 60 frames (approx 1 sec)
        if (animationFrameRef.current % 60 === 0) {
            console.log(`Perf (ms) -> Rust: ${(t1 - t0).toFixed(2)} | JS/Paint: ${(t2 - t1).toFixed(2)} | Total: ${(t2 - t0).toFixed(2)}`);
        }

        animationFrameRef.current = requestAnimationFrame(animate);
    };

    return (
        <div className="visualizer-container">
            <div className="controls">
                <label className="upload-btn">
                    Upload Image
                    <input type="file" accept="image/*" onChange={handleImageUpload} hidden />
                </label>
                <label className="upload-btn">
                    Upload Audio
                    <input type="file" accept="audio/*" onChange={handleAudioUpload} hidden />
                </label>
            </div>
            {hasAudio && (
                <div className="playback-controls">
                    <button className="control-btn" onClick={rewindAudio}>⏮ Rewind</button>
                    {!isPlaying ? (
                        <button className="control-btn" onClick={playAudio}>▶ Play</button>
                    ) : (
                        <button className="control-btn" onClick={pauseAudio}>⏸ Pause</button>
                    )}
                </div>
            )}

            <div className="settings-panel" style={{ marginTop: '10px', marginBottom: '10px' }}>
                <span style={{ marginRight: '10px', fontWeight: 'bold' }}>Colors:</span>
                {[4, 8, 16, 32, 64].map(count => (
                    <label key={count} style={{ marginRight: '10px', cursor: 'pointer' }}>
                        <input
                            type="radio"
                            name="colorCount"
                            value={count}
                            checked={colorCount === count}
                            onChange={() => handleColorCountChange(count)}
                            disabled={!visualizer}
                        />
                        <span style={{ marginLeft: '4px' }}>{count}</span>
                    </label>
                ))}
            </div>

            {!visualizer ? <p>Loading WASM...</p> : null}
            <canvas ref={canvasRef} className="visualizer-canvas" />
        </div>
    );
};
