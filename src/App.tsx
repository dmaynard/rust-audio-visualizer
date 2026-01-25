import { Visualizer } from './components/Visualizer'
import './App.css'

function App() {
  return (
    <div className="app-container">
      <header>
        <h1>Rust Audio Visualizer</h1>
        <p>WASM-powered Median Cut Quantization</p>
      </header>
      <main>
        <Visualizer />
      </main>
    </div>
  )
}

export default App
