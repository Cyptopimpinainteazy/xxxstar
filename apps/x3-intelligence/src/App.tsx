import { Routes, Route, NavLink } from 'react-router-dom'
import NetworkOverview from './views/NetworkOverview'
import CrossVmActivity from './views/CrossVmActivity'
import SupplyDashboard from './views/SupplyDashboard'
import SwarmActivity from './views/SwarmActivity'
import './App.css'

function App() {
  return (
    <div className="app-shell">
      <header className="app-header">
        <h1 className="app-title">x3-intelligence</h1>
        <nav className="app-nav">
          <NavLink to="/" end className={({ isActive }) => isActive ? 'nav-link active' : 'nav-link'}>
            Network Overview
          </NavLink>
          <NavLink to="/cross-vm" className={({ isActive }) => isActive ? 'nav-link active' : 'nav-link'}>
            Cross-VM Activity
          </NavLink>
          <NavLink to="/supply" className={({ isActive }) => isActive ? 'nav-link active' : 'nav-link'}>
            Supply Dashboard
          </NavLink>
          <NavLink to="/swarm" className={({ isActive }) => isActive ? 'nav-link active' : 'nav-link'}>
            Swarm Activity
          </NavLink>
        </nav>
      </header>
      <main className="app-main">
        <Routes>
          <Route path="/" element={<NetworkOverview />} />
          <Route path="/cross-vm" element={<CrossVmActivity />} />
          <Route path="/supply" element={<SupplyDashboard />} />
          <Route path="/swarm" element={<SwarmActivity />} />
        </Routes>
      </main>
    </div>
  )
}

export default App
