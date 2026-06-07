# GPU Swarm Dashboard

A modern, real-time monitoring dashboard for the X3 Chain GPU Swarm network. Built with React, TypeScript, and Tailwind CSS.

## Features

- **Real-time Metrics**: Live GPU utilization, memory, temperature, and power monitoring
- **Task Management**: Queue visualization and task lifecycle tracking
- **Network Topology**: Peer graph and network health monitoring
- **Economics Dashboard**: Rewards, staking, and slashing overview
- **Governance Interface**: View and vote on protocol proposals
- **System Health**: Coordinator and node status monitoring
- **Responsive Design**: Works on desktop and tablet devices

## Quick Start

### Prerequisites

- Node.js 16+
- npm or yarn

### Installation

```bash
cd apps/swarm-dashboard
npm install
```

### Development

```bash
npm run dev
```

The dashboard will be available at `http://localhost:5173`

### Build for Production

```bash
npm run build
```

Build output will be in the `dist/` directory.

## Configuration

Set the following environment variables to customize the dashboard:

```bash
VITE_API_BASE_URL=http://localhost:5000/api
VITE_WS_BASE_URL=ws://localhost:5000/ws
```

## Project Structure

```
src/
├── components/           # React components
│   ├── Layout.tsx         # Header, Sidebar, Footer
│   ├── Dashboard.tsx      # Main dashboard overview
│   ├── GpuMonitoring.tsx  # GPU device monitoring
│   ├── TaskManagement.tsx # Task queue and details
│   ├── NetworkTopology.tsx# Network peer visualization
│   ├── Economics.tsx      # Rewards and staking
│   ├── Governance.tsx     # Governance proposals
│   ├── Settings.tsx       # Configuration panel
│   └── Common.tsx         # Reusable UI components
├── hooks/                 # Custom React hooks
│   ├── useWebSocket.ts    # WebSocket connection
│   └── useQuery.ts        # Data fetching hooks
├── services/              # API client
│   └── api.ts             # API client implementation
├── types/                 # TypeScript types
│   └── api.ts             # API response types
├── utils/                 # Utility functions
├── App.tsx                # Main app component
├── main.tsx               # React root
└── index.css              # Global styles
```

## API Integration

The dashboard connects to the GPU Swarm backend via:

- **REST API**: `http://localhost:5000/api`
- **WebSocket**: `ws://localhost:5000/ws`

See [API Documentation](../../docs/API.md) for endpoint details.

## Technologies

- **React 18** - UI framework
- **TypeScript 5** - Type safety
- **Vite 5** - Build tool
- **Tailwind CSS 3** - Utility-first CSS
- **Recharts 2** - Chart library
- **TanStack React Query 5** - Data fetching
- **Zustand 4** - State management
- **React Router 6** - Routing

## Contributing

Please follow the project's contribution guidelines and code style.

## License

See [LICENSE](../../LICENSE) file for details.
