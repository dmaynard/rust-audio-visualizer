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
                <div style={{ marginTop: '20px', textAlign: 'left', fontSize: '0.9em' }}>
                    <p>Features:</p>
                    <ul>
                        <li>Hardware-accelerated WASM core</li>
                        <li>Spectral Equalizer with Hue Sorting</li>
                        <li>Dynamic Color Quantization</li>
                    </ul>
                </div>
            </div>
        </div>
    );
};
