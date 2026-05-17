import React from 'react';
import '../App.css'; // Re-use main styles or add specific ones

interface AboutProps {
    isOpen: boolean;
    onClose: () => void;
}

export const About: React.FC<AboutProps> = ({ isOpen, onClose }) => {
    if (!isOpen) return null;

    return (
        <div className="about-overlay" onClick={onClose}>
            <div className="about-modal" onClick={e => e.stopPropagation()}>
                <button className="close-btn" onClick={onClose}>×</button>
                <h2>Rust Audio Visualizer</h2>
                <p>WASM-powered Median Cut Quantization</p>
                
                <div className="about-content" style={{ marginTop: '20px', textAlign: 'left', fontSize: '0.9em', lineHeight: '1.5' }}>
                    <p>
                        This application is a high-performance audio visualizer built with React and Rust. It uses a custom WebAssembly engine to extract the dominant colors from any image using the Median Cut Quantization algorithm. Those colors are then dynamically pulsed and modulated to the frequencies of the audio track in real-time.
                    </p>
                    
                    <h3 style={{ marginTop: '15px', fontSize: '1em', color: '#ccc' }}>Features</h3>
                    <ul style={{ paddingLeft: '20px' }}>
                        <li>Hardware-accelerated zero-copy WASM core</li>
                        <li>Spectral Equalizer with true power band scaling</li>
                        <li>Dynamic Color Quantization and HSL Palette Animation</li>
                        <li>Drag-and-drop support for local audio and image files (including HEIC)</li>
                    </ul>

                    <h3 style={{ marginTop: '15px', fontSize: '1em', color: '#ccc' }}>Credits</h3>
                    <ul style={{ paddingLeft: '20px' }}>
                        <li><strong>Audio:</strong> Chopin - Polonaise in A Op.40 No.1 (Military) performed by eldüendesüarez (Public Domain via Musopen)</li>
                        <li><strong>Image:</strong> <em>L'atmosphère: météorologie populaire</em> by Camille Flammarion, 1888 (Colored version of the Flammarion engraving)</li>
                    </ul>

                    <div style={{ marginTop: '25px', textAlign: 'center' }}>
                        <a 
                            href="https://github.com/dmaynard/rust-audio-visualizer" 
                            target="_blank" 
                            rel="noopener noreferrer"
                            style={{ color: '#3498db', textDecoration: 'none', fontWeight: 'bold' }}
                        >
                            View Source on GitHub
                        </a>
                    </div>
                </div>
            </div>
        </div>
    );
};
