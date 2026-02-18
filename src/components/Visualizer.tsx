import React, { useEffect, useRef, useState } from 'react';
import init, { AudioVisualizer } from '../../crate/pkg/audio_visualizer_core';

export const Visualizer: React.FC = () => {
    const canvasRef = useRef<HTMLCanvasElement>(null);

    // Flag to prevent recursive calls to WASM during image load
    const isImageLoading = useRef(false);

    const [visualizer, setVisualizer] = useState<AudioVisualizer | null>(null);
    const [wasmMemory, setWasmMemory] = useState<WebAssembly.Memory | null>(null);
    const sourceRef = useRef<AudioBufferSourceNode | MediaStreamAudioSourceNode | null>(null);
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

    // Use ref for mic stream to handle cleanup without re-renders affecting it immediately
    const micStreamRef = useRef<MediaStream | null>(null);
    const [isMicActive, setIsMicActive] = useState(false);

    // Microphone Selection
    interface AudioDevice {
        deviceId: string;
        label: string;
    }
    const [audioDevices, setAudioDevices] = useState<AudioDevice[]>([]);
    const [selectedDeviceId, setSelectedDeviceId] = useState<string>('');

    useEffect(() => {
        // Animation should run if File Playing OR Mic Active
        isPlayingRef.current = isPlaying || isMicActive;

        // If we became active and loop isn't running, start it
        if ((isPlaying || isMicActive) && !animationFrameRef.current) {
            animate();
        }
    }, [isPlaying, isMicActive]);

    // Fetch Audio Devices on Mount
    useEffect(() => {
        const fetchDevices = async () => {
            try {
                // Check permission first (optional, but helps get labels)
                // await navigator.mediaDevices.getUserMedia({ audio: true }); 

                const devices = await navigator.mediaDevices.enumerateDevices();
                const audioInputs = devices
                    .filter(device => device.kind === 'audioinput')
                    .map(device => ({
                        deviceId: device.deviceId,
                        label: device.label || `Microphone ${device.deviceId.slice(0, 5)}...`
                    }));

                setAudioDevices(audioInputs);
                if (audioInputs.length > 0 && !selectedDeviceId) {
                    setSelectedDeviceId(audioInputs[0].deviceId);
                }
            } catch (e) {
                console.error("Error fetching audio devices:", e);
            }
        };
        fetchDevices();

        // Listen for device changes
        navigator.mediaDevices.addEventListener('devicechange', fetchDevices);
        return () => navigator.mediaDevices.removeEventListener('devicechange', fetchDevices);
    }, []);

    const isTogglingRef = useRef(false);

    const toggleMic = async () => {
        if (isTogglingRef.current) return;
        isTogglingRef.current = true;
        console.log("Visualizer: toggleMic called. Current state:", isMicActive);

        try {
            if (isMicActive) {
                // STOP MIC
                console.log("Visualizer: Stopping Mic");
                if (micStreamRef.current) {
                    micStreamRef.current.getTracks().forEach(track => track.stop());
                    micStreamRef.current = null;
                }
                stopAudio(); // Disconnects source and stops sourceRef
                setIsMicActive(false);
            } else {
                // START MIC
                console.log("Visualizer: Starting Mic");
                // Request Mic Permission with specific device if selected
                const constraints = {
                    audio: selectedDeviceId ? { deviceId: { exact: selectedDeviceId } } : true
                };

                const stream = await navigator.mediaDevices.getUserMedia(constraints);
                console.log("Visualizer: Mic Stream acquired:", stream.id);
                micStreamRef.current = stream;

                // Ensure Audio Context is Ready
                if (!audioContextRef.current) {
                    audioContextRef.current = new (window.AudioContext || (window as any).webkitAudioContext)();
                }
                if (audioContextRef.current.state === 'suspended') {
                    await audioContextRef.current.resume();
                }

                // Stop any file playback
                if (isPlaying) {
                    pauseAudio();
                } else {
                    stopAudio();
                }

                const ctx = audioContextRef.current;
                const source = ctx.createMediaStreamSource(stream);

                if (!analyserRef.current) {
                    const analyser = ctx.createAnalyser();
                    analyser.fftSize = 2048;
                    analyserRef.current = analyser;
                }

                source.connect(analyserRef.current);
                // DO NOT connect to destination (speakers) to avoid feedback!

                sourceRef.current = source;

                if (visualizer) {
                    visualizer.resize_input_buffer(analyserRef.current.frequencyBinCount);
                }

                setIsMicActive(true);
                animate();
            }
        } catch (err) {
            console.error("Error accessing microphone:", err);
            alert("Could not access microphone. See console.");
        } finally {
            isTogglingRef.current = false;
        }
    };

    const handleColorCountChange = (count: number) => {
        setColorCount(count);
        if (visualizer) {
            try {
                isImageLoading.current = true;
                visualizer.set_color_count(count);
                isImageLoading.current = false;

                // Force a re-render of the frame to show new palette immediately
                if (visualizer.get_width() > 0) {
                    renderFrame();
                }
            } catch (e) {
                console.error("Error setting color count:", e);
                isImageLoading.current = false;
            }
        }
    };

    useEffect(() => {
        const loadWasm = async () => {
            const module = await init();
            setWasmMemory(module.memory);
            console.log("Visualizer: WASM module loaded (v4 Tuned: Zero-Copy Upload)");
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
            // Check if source has a stop method (BufferSource) vs StreamSource
            if ('stop' in sourceRef.current) {
                try {
                    (sourceRef.current as AudioBufferSourceNode).stop();
                } catch (e) { /* ignore */ }
            }
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
            analyser.fftSize = 2048;
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

        // FORCE A RESET FRAME:
        // When we pause, we want to reset the visualizer to the "silence" state (original palette)
        if (visualizer && wasmMemory && analyserRef.current) {
            const bufferLength = analyserRef.current.frequencyBinCount;
            const inputPtr = visualizer.get_input_buffer_ptr();

            // 1. Create a zero-filled buffer
            const zeros = new Uint8Array(bufferLength).fill(0);

            // 2. Write to WASM memory
            const wasmInputArray = new Uint8Array(wasmMemory.buffer, inputPtr, bufferLength);
            wasmInputArray.set(zeros);

            // 3. Process and Render one frame
            visualizer.process_frequencies();
            visualizer.render();
            renderFrame();
        }
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

    // Helper: Process Image from URL (Blob or String)
    const processImage = (url: string) => {
        const img = new Image();
        img.onload = () => {
            // Calculate dimensions satisfying max 1600x1200
            const MAX_W = 1600;
            const MAX_H = 1200;
            let w = img.width;
            let h = img.height;

            // Lanczos-like scaling logic
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
                // Pause animation loop to prevent "recursive use of object" error
                isImageLoading.current = true;

                if (!visualizer || !wasmMemory) throw new Error("WASM not ready");

                // 1. Get pointer to Rust's internal upload buffer
                const uploadPtr = visualizer.get_upload_buffer_ptr();
                if (uploadPtr === 0) throw new Error("Failed to get upload buffer pointer");

                // 2. Copy image data directly into WASM memory (Zero Allocation on WASM side)
                const wasmUploadBuffer = new Uint8Array(wasmMemory.buffer, uploadPtr, data.length);
                wasmUploadBuffer.set(data);

                // 3. Trigger processing (no data argument needed)
                visualizer.load_image(w, h);

                resizeCanvas();
                renderFrame();

                // Resume animation loop
                isImageLoading.current = false;

            } catch (err) {
                console.error("Error loading image:", err);
                // alert("Error loading image. Check console."); 
                // Suppress alert for default load, log only
                isImageLoading.current = false;
            }
        };
        img.onerror = () => {
            console.error("Failed to load image url:", url);
        };
        img.src = url;
    };

    const handleImageUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
        if (!e.target.files || !e.target.files[0] || !visualizer) return;
        const file = e.target.files[0];
        const url = URL.createObjectURL(file);
        processImage(url);
        // Note: We don't revokeObjectURL here inside the helper because we might need it for defaults? 
        // Actually, for file uploads we should revoke. 
        // Modified processImage to NOT revoke. Caller handles it? 
        // Or just let GC handle it for now.
    };

    // Helper: Process Audio Buffer
    const processAudio = async (buffer: ArrayBuffer) => {
        if (!audioContextRef.current) {
            audioContextRef.current = new (window.AudioContext || (window as any).webkitAudioContext)();
        }

        const ctx = audioContextRef.current;
        try {
            const audioBuffer = await ctx.decodeAudioData(buffer);
            audioBufferRef.current = audioBuffer;

            setHasAudio(true);
            setPausedAt(0);
            setStartTime(0);
            // playAudio(); // Don't auto-play defaults to avoid permission errors
        } catch (e) {
            console.error("Error decoding audio:", e);
        }
    };

    const handleAudioUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
        if (!e.target.files || !e.target.files[0]) return;

        // If Mic is active, stop it
        if (isMicActive) {

            if (micStreamRef.current) {
                micStreamRef.current.getTracks().forEach(track => track.stop());
                micStreamRef.current = null;
            }
            setIsMicActive(false);
        }

        const file = e.target.files[0];
        const buffer = await file.arrayBuffer();
        await processAudio(buffer);
        playAudio(); // Auto-play for user uploads
    };

    // Load Defaults Once WASM is Ready
    useEffect(() => {
        if (visualizer && wasmMemory) {
            const loadDefaults = async () => {
                // Load Image
                processImage("FlammarionColor.png");

                // Load Audio
                try {
                    const response = await fetch("Chopin_-_Polonaise_in_A_Op-40_No-1_(Military)_(Piano_Performance_by_eldüendesüarez).mp3");
                    if (response.ok) {
                        const buffer = await response.arrayBuffer();
                        await processAudio(buffer);
                    }
                } catch (e) {
                    console.error("Failed to load default audio:", e);
                }
            };
            loadDefaults();
        }
    }, [visualizer, wasmMemory]);

    const resizeCanvas = () => {
        if (!visualizer || !canvasRef.current) return;
        canvasRef.current.width = visualizer.get_width();
        canvasRef.current.height = visualizer.get_height();
    };

    const [viewMode, setViewMode] = useState<'image' | 'equalizer'>('image');
    const viewModeRef = useRef<'image' | 'equalizer'>('image');

    // Sync Ref
    useEffect(() => {
        viewModeRef.current = viewMode;
        // Trigger manual render when mode changes (for paused state)
        if (visualizer && wasmMemory) {
            renderFrame();
        }
    }, [viewMode, visualizer, wasmMemory]);

    const renderEqualizer = () => {
        if (!visualizer || !canvasRef.current || !wasmMemory) return;
        const ctx = canvasRef.current.getContext('2d');
        if (!ctx) return;

        // Match canvas resolution to display size for crisp equalizer
        const displayWidth = canvasRef.current.clientWidth;
        const displayHeight = canvasRef.current.clientHeight;

        if (canvasRef.current.width !== displayWidth || canvasRef.current.height !== displayHeight) {
            canvasRef.current.width = displayWidth;
            canvasRef.current.height = displayHeight;
        }

        const width = canvasRef.current.width;
        const height = canvasRef.current.height;

        // Clear canvas for fresh draw
        ctx.fillStyle = "#111"; // Dark background
        ctx.fillRect(0, 0, width, height);

        const spectrumPtr = visualizer.get_spectrum_ptr();
        const palettePtr = visualizer.get_palette_ptr();

        // We have `colorCount` bins (e.g. 64, 128, 256)
        // Access raw memory
        const spectrum = new Float32Array(wasmMemory.buffer, spectrumPtr, colorCount);
        const palette = new Uint8Array(wasmMemory.buffer, palettePtr, colorCount * 3);

        const barWidth = width / colorCount;

        for (let i = 0; i < colorCount; i++) {
            const energy = spectrum[i]; // 0.0 to 1.0 (approx)
            const barHeight = energy * height * 0.8; // Scale to 80% height

            const r = palette[i * 3];
            const g = palette[i * 3 + 1];
            const b = palette[i * 3 + 2];

            ctx.fillStyle = `rgb(${r}, ${g}, ${b})`;

            // Draw bar from bottom
            ctx.fillRect(
                i * barWidth,
                height - barHeight,
                barWidth - 1, // -1 for gap
                barHeight
            );
        }
    };

    const renderFrame = () => {
        if (!visualizer || !canvasRef.current || !wasmMemory) return;
        // visualizer.render() is called inside animate for audio, or manually for static image

        if (viewModeRef.current === 'image') {
            const width = visualizer.get_width();
            const height = visualizer.get_height();
            if (width === 0 || height === 0) return;

            // Ensure canvas matches image resolution
            if (canvasRef.current.width !== width || canvasRef.current.height !== height) {
                canvasRef.current.width = width;
                canvasRef.current.height = height;
            }

            const bufferPtr = visualizer.get_display_buffer_ptr();
            const len = visualizer.get_display_buffer_len();

            const memBuffer = new Uint8Array(wasmMemory.buffer);
            const imageBuffer = new Uint8ClampedArray(memBuffer.subarray(bufferPtr, bufferPtr + len));
            const imageData = new ImageData(imageBuffer, width, height);

            const ctx = canvasRef.current.getContext('2d');
            if (ctx) {
                // Ensure canvas size matches image size (it should already)
                // ctx.putImageData(imageData, 0, 0); 
                // Draw image scaled to canvas if needed, but putImageData is 1:1.
                // If viewMode changed, canvas size might be different? 
                // We should keep canvas size = image size for image mode. 
                // For equalizer mode, we can use same canvas size.
                ctx.putImageData(imageData, 0, 0);
            }
        } else {
            renderEqualizer();
        }
    };

    const animate = () => {
        if (!analyserRef.current || !visualizer || !wasmMemory) return;

        // Stop loop if not playing (checked via Ref to avoid stale closure)
        if (!isPlayingRef.current) {
            animationFrameRef.current = 0;
            return;
        }

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
        // const t0 = performance.now();
        try {
            visualizer.process_frequencies();
            visualizer.render();
        } catch (e) {
            console.error("Rust execution error:", e);
        }
        // const t1 = performance.now();

        // 6. Measure Render/Paint Time
        renderFrame();
        // const t2 = performance.now();

        // Log performance stats every 60 frames (approx 1 sec)
        // if (animationFrameRef.current % 60 === 0) {
        //     console.log(`Perf(ms) -> Rust: ${ (t1 - t0).toFixed(2) } | JS / Paint: ${ (t2 - t1).toFixed(2) } | Total: ${ (t2 - t0).toFixed(2) } `);
        // }

        animationFrameRef.current = requestAnimationFrame(animate);
    };

    return (
        <div className="visualizer-container">
            <div className="controls">
                <label className="upload-btn">
                    Load Image
                    <input type="file" accept="image/*" onChange={handleImageUpload} hidden />
                </label>
                <label className="upload-btn">
                    Load Audio
                    <input type="file" accept="audio/*" onChange={handleAudioUpload} hidden />
                </label>
            </div>
            <div className="playback-controls" style={{ marginTop: '10px' }}>
                {hasAudio && (
                    <>
                        <button className="control-btn" onClick={rewindAudio}>⏮ Rewind</button>
                        {!isPlaying ? (
                            <button className="control-btn" onClick={playAudio}>▶ Play</button>
                        ) : (
                            <button className="control-btn" onClick={pauseAudio}>⏸ Pause</button>
                        )}
                    </>
                )}

                <button
                    className="control-btn"
                    onClick={toggleMic}
                    style={{ backgroundColor: isMicActive ? '#e74c3c' : '', marginLeft: hasAudio ? '10px' : '0' }}
                >
                    {isMicActive ? "⏹ Stop Mic" : "🎤 Start Mic"}
                </button>

                <button
                    className="control-btn"
                    onClick={() => setViewMode(prev => prev === 'image' ? 'equalizer' : 'image')}
                    style={{ marginLeft: '10px' }}
                >
                    {viewMode === 'image' ? "📊 Chart" : "🖼️ Image"}
                </button>

                {audioDevices.length > 0 && (
                    <select
                        style={{ marginLeft: '10px', padding: '5px' }}
                        value={selectedDeviceId}
                        onChange={(e) => setSelectedDeviceId(e.target.value)}
                        disabled={isMicActive}
                    >
                        {audioDevices.map(device => (
                            <option key={device.deviceId} value={device.deviceId}>
                                {device.label}
                            </option>
                        ))}
                    </select>
                )}
            </div>

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
