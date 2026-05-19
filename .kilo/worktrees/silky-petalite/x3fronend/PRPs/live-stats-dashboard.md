name: "Live Stats Dashboard - Real-time X3 Chain Statistics"
description: |

## Purpose
Build a Live Stats Dashboard component that displays real-time X3 Chain blockchain statistics with animated counters and a live feed of recent transactions. The dashboard should show validator count, TPS (transactions per second), block height, and recent transaction activity with auto-refresh every 5 seconds.

## Core Principles
1. **Context is King**: Include ALL necessary documentation, examples, and caveats
2. **Validation Loops**: Provide executable tests/lints the AI can run and fix
3. **Information Dense**: Use keywords and patterns from the codebase
4. **Progressive Success**: Start simple, validate, then enhance
5. **Global rules**: Be sure to follow all rules in CLAUDE.md

---

## Goal
Build a React component (`LiveStatsDashboard`) that displays real-time X3 Chain blockchain statistics with animated number counters, live transaction feed, and auto-refresh capabilities. The component should integrate seamlessly with the existing X3STAR frontend codebase.

## Why
- **Business value**: Provides users with real-time visibility into X3 Chain network health and activity
- **User impact**: Builds trust through transparency and live data
- **Integration**: Extends the existing Landing page with a dedicated stats section
- **Problems solved**: Users currently have no centralized place to see live blockchain metrics

## What
User-visible behavior and technical requirements:
- Animated number counters for: Validator Count, TPS, Block Height, Total Value Locked
- Live transaction feed showing recent transactions with timestamps
- Auto-refresh every 5 seconds using polling
- Loading states with skeleton UI
- Error handling with retry functionality
- Responsive design (mobile-first)
- Integration with existing `data/business-store.json` for initial data
- Follows existing color scheme: cyan (#00e5ff), gold (#ffd700), ink (#000008)
- Uses gradient-text and badge styling from Landing.jsx

### Success Criteria
- [ ] Component renders without errors
- [ ] Animated counters display with smooth transitions
- [ ] Live feed updates every 5 seconds
- [ ] Loading states display correctly
- [ ] Error states display with retry button
- [ ] Responsive on mobile and desktop
- [ ] Passes Playwright smoke test
- [ ] No console errors

## All Needed Context

### Documentation & References
```yaml
# MUST READ - Include these in your context window
- file: src/components/Landing.jsx
  why: Existing component pattern with hooks (useEffect, useRef), Three.js integration, Tailwind CSS styling, gradient-text class, badge styling
  
- file: js/x3-data-api.js
  why: Existing data fetching pattern with caching, error handling, subscribe function for SSE
  
- file: data/business-store.json
  why: Data structure with networkTelemetry.validators array, token stats, staking stats
  
- file: src/components/Button.jsx
  why: Reusable button component pattern
  
- file: src/components/HexRing.jsx
  why: Animated visual component pattern
  
- file: tests/playwright-smoke.js
  why: Existing test pattern for Playwright E2E tests
  
- file: package.json
  why: Dependencies (react, three, tailwindcss, vite)
  
- file: vite.config.js
  why: Build configuration
  
- file: tailwind.config.js
  why: Custom colors, animations, and theme configuration
```

### Current Codebase tree
```bash
x3fronend/
├── src/
│   ├── components/
│   │   ├── Landing.jsx
│   │   ├── Button.jsx
│   │   ├── HexRing.jsx
│   │   ├── ScrollIndicator.jsx
│   │   ├── Eyebrow.jsx
│   │   └── ListItem.jsx
│   ├── App.jsx
│   ├── main.jsx
│   └── index.css
├── js/
│   ├── x3-data-api.js
│   ├── x3-page-adapters.js
│   └── x3-site-nav.js
├── data/
│   └── business-store.json
├── css/
│   └── x3-site-nav.css
├── tests/
│   ├── playwright-smoke.js
│   └── server.test.js
├── package.json
├── vite.config.js
├── tailwind.config.js
└── postcss.config.js
```

### Desired Codebase tree with files to be added
```bash
x3fronend/
├── src/
│   ├── components/
│   │   ├── Landing.jsx (existing)
│   │   ├── LiveStatsDashboard.jsx (NEW)
│   │   ├── AnimatedCounter.jsx (NEW)
│   │   ├── LiveTransactionFeed.jsx (NEW)
│   │   └── StatCard.jsx (NEW)
│   └── hooks/
│       └── useLiveStats.js (NEW)
├── tests/
│   └── live-stats-dashboard.spec.js (NEW)
```

### Known Gotchas of our codebase & Library Quirks
```javascript
// CRITICAL: React hooks must be called at top level, not inside conditions
// CRITICAL: useEffect cleanup required for intervals to prevent memory leaks
// CRITICAL: Three.js renderer must be disposed in cleanup
// CRITICAL: Tailwind classes must be defined in tailwind.config.js for custom values
// CRITICAL: data/business-store.json is static - real data comes from X3API
// CRITICAL: X3API.subscribe uses EventSource for SSE, not polling
// CRITICAL: Component files use .jsx extension, not .js
// CRITICAL: CSS uses custom properties like --ripple, --easing
// CRITICAL: gradient-text class is defined in index.css
// CRITICAL: Animations use requestAnimationFrame for smooth 60fps
```

## Implementation Blueprint

### Data models and structure
```javascript
// StatCard props
PropTypes: {
  label: PropTypes.string.isRequired,
  value: PropTypes.oneOfType([PropTypes.number, PropTypes.string]).isRequired,
  prefix: PropTypes.string,
  suffix: PropTypes.string,
  trend: PropTypes.number, // percentage change
  loading: PropTypes.bool,
  error: PropTypes.string
}

// LiveTransactionFeed item
TransactionShape: {
  id: string,
  type: 'BUY' | 'SELL' | 'STAKE' | 'VOTE' | 'MOVE',
  wallet: string,
  amountX3S: number,
  amountUsd: number,
  timestamp: string, // ISO date
  detail: string
}

// useLiveStats return shape
LiveStatsShape: {
  validators: number,
  tps: number,
  blockHeight: number,
  tvlUsd: number,
  recentTransactions: Array<TransactionShape>,
  loading: boolean,
  error: string | null,
  refresh: () => void
}
```

### List of tasks to be completed

```yaml
Task 1:
CREATE src/hooks/useLiveStats.js:
  - Implement custom hook using useState, useEffect
  - Fetch data from business-store.json initially
  - Set up 5-second polling interval
  - Handle loading/error states
  - MIRROR pattern from: js/x3-data-api.js (caching, error handling)

Task 2:
CREATE src/components/AnimatedCounter.jsx:
  - Implement animated number counter using requestAnimationFrame
  - Accept value, duration, prefix, suffix props
  - Animate from 0 to target value
  - Format numbers with commas
  - MIRROR pattern from: src/components/HexRing.jsx (animation pattern)

Task 3:
CREATE src/components/StatCard.jsx:
  - Create card layout with label, animated value, trend indicator
  - Use Tailwind CSS styling
  - Support loading skeleton state
  - Use gradient-text for values
  - MIRROR pattern from: src/components/ListItem.jsx (card pattern)

Task 4:
CREATE src/components/LiveTransactionFeed.jsx:
  - Render list of recent transactions
  - Show transaction type, wallet, amount, timestamp
  - Animate new entries with fade-in
  - Support empty state
  - MIRROR pattern from: src/components/Landing.jsx (list rendering)

Task 5:
CREATE src/components/LiveStatsDashboard.jsx:
  - Compose StatCard and LiveTransactionFeed components
  - Use useLiveStats hook
  - Add section header with Eyebrow component
  - Add refresh button
  - MIRROR pattern from: src/components/Landing.jsx (section structure)

Task 6:
CREATE tests/live-stats-dashboard.spec.js:
  - Test component renders
  - Test loading state displays
  - Test error state with retry
  - Test auto-refresh
  - MIRROR pattern from: tests/playwright-smoke.js

Task 7:
MODIFY src/App.jsx:
  - FIND pattern: "<Landing />"
  - INJECT after: "<Landing />"
  - ADD: "<LiveStatsDashboard />"
  - PRESERVE existing imports
```

### Per task pseudocode

```javascript
// Task 1 - useLiveStats.js
function useLiveStats(refreshInterval = 5000) {
  const [stats, setStats] = useState({
    validators: 0,
    tps: 0,
    blockHeight: 0,
    tvlUsd: 0,
    recentTransactions: []
  });
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  
  useEffect(() => {
    const fetchData = async () => {
      try {
        // PATTERN: Use fetch with error handling (see x3-data-api.js)
        const response = await fetch('/api/site/dashboard');
        if (!response.ok) throw new Error('Failed to fetch');
        const data = await response.json();
        
        // GOTCHA: business-store.json structure has nested objects
        setStats({
          validators: data.networkTelemetry?.validators?.length || 42,
          tps: calculateAvgTps(data.networkTelemetry?.validators),
          blockHeight: data.dashboard?.blockNumber || 1847341,
          tvlUsd: data.staking?.totalValueLockedUsd || 48200000,
          recentTransactions: data.marketWhales?.events || []
        });
        setError(null);
      } catch (err) {
        setError(err.message);
      } finally {
        setLoading(false);
      }
    };
    
    fetchData();
    // CRITICAL: Clear interval in cleanup to prevent memory leaks
    const interval = setInterval(fetchData, refreshInterval);
    return () => clearInterval(interval);
  }, [refreshInterval]);
  
  return { ...stats, loading, error, refresh: fetchData };
}

// Task 2 - AnimatedCounter.jsx
function AnimatedCounter({ value, duration = 2000, prefix = '', suffix = '' }) {
  const [displayValue, setDisplayValue] = useState(0);
  const ref = useRef(null);
  
  useEffect(() => {
    const start = 0;
    const end = value;
    const startTime = performance.now();
    
    // PATTERN: Use requestAnimationFrame for smooth animation (see Landing.jsx)
    const animate = (currentTime) => {
      const elapsed = currentTime - startTime;
      const progress = Math.min(elapsed / duration, 1);
      
      // Easing function for smooth animation
      const eased = 1 - Math.pow(1 - progress, 3);
      setDisplayValue(Math.floor(start + (end - start) * eased));
      
      if (progress < 1) {
        ref.current = requestAnimationFrame(animate);
      }
    };
    
    ref.current = requestAnimationFrame(animate);
    return () => cancelAnimationFrame(ref.current);
  }, [value, duration]);
  
  // Format with commas
  const formatted = displayValue.toLocaleString();
  return <span>{prefix}{formatted}{suffix}</span>;
}

// Task 3 - StatCard.jsx
function StatCard({ label, value, prefix, suffix, trend, loading, error }) {
  if (loading) {
    return <div className="stat-card skeleton animate-pulse" />;
  }
  
  if (error) {
    return <div className="stat-card error">{error}</div>;
  }
  
  return (
    <div className="stat-card bg-surface-dark rounded-2xl p-6">
      <p className="text-xs uppercase tracking-widest opacity-60 mb-2">{label}</p>
      <p className="display gradient-text text-4xl">
        <AnimatedCounter value={value} prefix={prefix} suffix={suffix} />
      </p>
      {trend && (
        <p className={`text-sm mt-2 ${trend > 0 ? 'text-green-400' : 'text-red-400'}`}>
          {trend > 0 ? '↑' : '↓'} {Math.abs(trend)}%
        </p>
      )}
    </div>
  );
}
```

### Integration Points
```yaml
COMPONENTS:
  - add to: src/components/
  - pattern: "export default function ComponentName(props) { ... }"
  
STYLES:
  - add to: src/index.css
  - pattern: ".stat-card { @apply bg-surface-dark rounded-2xl p-6; }"
  
ROUTES:
  - add to: src/App.jsx
  - pattern: "<LiveStatsDashboard />"
  
DATA:
  - initial: data/business-store.json (static fallback)
  - live: X3API.getDashboardData() or fetch('/api/site/dashboard')
  - refresh: 5000ms interval
```

## Validation Loop

### Level 1: Syntax & Style
```bash
# Run these FIRST - fix any errors before proceeding
npm run lint  # ESLint check

# Expected: No errors. If errors, READ the error and fix.
```

### Level 2: Unit Tests
```javascript
// CREATE tests/live-stats-dashboard.spec.js
test('renders without crashing', async ({ page }) => {
  await page.goto('http://localhost:5173');
  await expect(page.locator('.live-stats-dashboard')).toBeVisible();
});

test('displays loading state', async ({ page }) => {
  await page.goto('http://localhost:5173');
  await expect(page.locator('.skeleton')).toBeVisible();
});

test('displays stats after load', async ({ page }) => {
  await page.goto('http://localhost:5173');
  await page.waitForSelector('.stat-card:not(.skeleton)', { timeout: 10000 });
  await expect(page.locator('.stat-card')).toHaveCount(4);
});

test('shows error state with retry', async ({ page }) => {
  // Mock API failure
  await page.route('**/api/site/dashboard', route => route.abort());
  await page.goto('http://localhost:5173');
  await expect(page.locator('.error')).toBeVisible();
  await expect(page.locator('button:has-text("Retry")')).toBeVisible();
});
```

```bash
# Run and iterate until passing:
npx playwright test tests/live-stats-dashboard.spec.js
# If failing: Read error, understand root cause, fix code, re-run
```

### Level 3: Integration Test
```bash
# Start the development server
npm run dev

# Test the component in browser
open http://localhost:5173

# Expected: Dashboard displays with animated counters
# If error: Check browser console for stack trace
```

## Final validation Checklist
- [ ] All tests pass: `npx playwright test`
- [ ] No linting errors: `npm run lint`
- [ ] Build succeeds: `npm run build`
- [ ] Manual test successful: Dashboard displays in browser
- [ ] Error cases handled gracefully
- [ ] Logs are informative but not verbose
- [ ] Documentation updated if needed

---

## Anti-Patterns to Avoid
- ❌ Don't create new patterns when existing ones work
- ❌ Don't skip validation because "it should work"  
- ❌ Don't ignore failing tests - fix them
- ❌ Don't use class components when functional components with hooks exist
- ❌ Don't hardcode values that should be config
- ❌ Don't catch all exceptions - be specific
- ❌ Don't forget to clear intervals in useEffect cleanup
- ❌ Don't mix CSS-in-JS with Tailwind classes