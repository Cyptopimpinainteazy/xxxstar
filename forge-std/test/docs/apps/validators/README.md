# Validators Dashboard

A real-time validator network monitoring and analytics dashboard built with React and Vite.

## Features

- **Interactive 3D Globe Visualization**: Rotating globe showing validator node locations worldwide
- **Live Network Status**: Real-time validator status (online, syncing, offline)
- **Node Metrics**: View validator score, blocks validated, uptime, and coordinates
- **Network Connections**: Visual representation of validator inter-node connections
- **Interactive Details**: Click on validator nodes to see detailed performance metrics
- **Drag to Rotate**: Manually rotate the globe or let it auto-rotate

## Quick Start

### Development

```bash
npm install
npm run dev
```

The app will be available at `http://localhost:3013`

### Build

```bash
npm run build
npm run preview
```

## Structure

```
src/
├── components/
│   └── ValidatorGlobe.tsx    # Main interactive globe component
├── App.tsx                   # Root application component
├── main.tsx                  # Entry point
└── index.css                 # Global styles
```

## Technologies

- **React 18** - UI framework
- **Vite** - Build tool and dev server
- **Framer Motion** - Smooth animations
- **Tailwind CSS** - Styling
- **Canvas API** - Globe rendering
- **SVG** - Connection visualization

## Usage

### Starting the Development Server

```bash
npm run dev
```

This will start the Vite development server on port 3013.

### Building for Production

```bash
npm run build
```

The build output will be in the `dist/` directory.

### Integration with X3 Desktop

The Validators app is registered in the desktop launcher at `apps/x3-desktop/src/services/applicationService.ts` and can be launched from the desktop UI.

## Configuration

- **Port**: 3013 (configurable in `vite.config.ts`)
- **Globe cities**: 20 major global cities (configurable in `ValidatorGlobe.tsx`)
- **Update interval**: 2 seconds for node metrics updates

## Future Enhancements

- Real data integration from blockchain RPC
- Staking information and rewards
- Historical validator performance analytics
- Delegation management
- Governance participation tracking
- Advanced filtering and search
- Export metrics to CSV/JSON

## License

Part of the X3 Chain project.
