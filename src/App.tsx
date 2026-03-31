import { useState } from 'react'
import { Visualizer } from './components/Visualizer'
import { About } from './components/About'
import './App.css'

function App() {
  const [showAbout, setShowAbout] = useState(false);

  return (
    <div className="app-container">
      <button
        className="info-btn"
        onClick={() => setShowAbout(true)}
        title="About / Info"
      >
        ℹ️
      </button>

      <About isOpen={showAbout} onClose={() => setShowAbout(false)} />

      <main>
        <Visualizer />
      </main>
    </div>
  )
}

export default App
