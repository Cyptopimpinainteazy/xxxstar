import { panelRegistry } from './panelRegistry'

function App() {
  return (
    <div>
      <h1>X3 Dashboard</h1>
      <p>{Object.keys(panelRegistry).length} panels available</p>
    </div>
  )
}

export default App
