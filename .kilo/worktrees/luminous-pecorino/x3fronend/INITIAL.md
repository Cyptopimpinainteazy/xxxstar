## FEATURE:

Build a Live Stats Dashboard component that displays real-time X3 Chain blockchain statistics with animated counters and a live feed of recent transactions. The dashboard should show validator count, TPS (transactions per second), block height, and recent transaction activity with auto-refresh every 5 seconds.

## EXAMPLES:

- `src/components/Landing.jsx` - Shows the existing component pattern with Three.js integration, hooks usage (useEffect, useRef), and Tailwind CSS styling
- `src/components/Button.jsx` - Reusable button component pattern
- `src/components/HexRing.jsx` - Animated visual component pattern
- `src/services/x3-data-api.js` - Existing data fetching pattern to follow

## DOCUMENTATION:

- React hooks documentation: https://react.dev/reference/react
- Tailwind CSS animations: https://tailwindcss.com/docs/animation
- Three.js for 3D elements: https://threejs.org/docs/
- X3 Chain API endpoints: Use existing `js/x3-data-api.js` patterns

## OTHER CONSIDERATIONS:

- Must integrate with existing `data/business-store.json` for initial data
- Follow the gradient-text and badge styling from Landing.jsx
- Use the same color scheme: cyan (#00e5ff), gold (#ffd700), ink (#000008)
- Component should be responsive (mobile-first)
- Add loading states and error handling for API failures
- Include Playwright tests in `tests/` directory