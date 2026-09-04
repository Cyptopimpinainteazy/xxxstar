# Inferstructor Dashboard Testing Guide

## Smoke Tests - Manual Testing Checklist

### Authentication Flow
- [ ] User can navigate to register page
- [ ] User can fill registration form and submit
- [ ] User is redirected to overview page after successful registration
- [ ] User can logout from authenticated state

### Operator Pages

#### Overview (Dashboard)
- [ ] Overview page loads with metrics summary
- [ ] Summary cards display correctly (TPS, latency, uptime, gas efficiency)
- [ ] TpsLeaderboard component is accessible from overview

#### Validators (Operator)
- [ ] Validators page loads
- [ ] ValidatorControls component renders properly
- [ ] Search by name/ID filters validators correctly
- [ ] Status badges display correct colors (approved/pending/suspended)

#### Swaps, Proofs, Faucet, Funding, Settings
- [ ] Each page loads without errors
- [ ] Placeholder content displays correctly
- [ ] Navigation remains functional

### Navigation

#### Top Navigation Bar
- [ ] Logo/brand name displays correctly
- [ ] Breadcrumbs update when navigating between pages
- [ ] Admin button is accessible
- [ ] Logout button functions properly

#### Sidebar
- [ ] Sidebar opens and closes smoothly
- [ ] All menu items are visible when sidebar is expanded
- [ ] Current page is highlighted
- [ ] Icons render correctly with lucide-react

#### Responsive Design
- [ ] Sidebar collapses on smaller screens
- [ ] Content area adjusts margin (ml-64) properly
- [ ] All components remain readable on mobile viewport

### Admin Pages

#### Admin Dashboard
- [ ] Admin login page accessible via Admin button
- [ ] Admin mode displays different navigation menu
- [ ] Admin dashboard loads correctly

#### Validator Controls
- [ ] Admin can view list of validators
- [ ] Search functionality works
- [ ] Approve/Suspend/Unlock actions are accessible
- [ ] Status updates persist across page refreshes

#### Admin Controls (Multi-tab)
- [ ] RPC Endpoints tab shows endpoint list
- [ ] Faucet Config tab allows rate limit input
- [ ] Emergency toggle switches state correctly
- [ ] RBAC tab displays roles and permissions matrix
- [ ] Audit Logs tab shows transaction history
- [ ] Tab switching works smoothly

#### Leaderboard & Metrics
- [ ] Summary metrics cards display with correct values
- [ ] Hourly snapshots table populates
- [ ] Sort by TPS/Latency/Uptime/Gas Efficiency works
- [ ] Filter by chain (Ethereum/Solana) works
- [ ] Export CSV button downloads file
- [ ] Add Snapshot button adds new row (Admin Mode)
- [ ] Admin Mode toggle works
- [ ] Snapshots persist after page refresh (localStorage)

### Data Persistence

- [ ] Metrics snapshots persist in localStorage
- [ ] Snapshots survive page reload
- [ ] CSV export includes all snapshots
- [ ] No console errors appear

### Error Handling

- [ ] No JavaScript errors in console
- [ ] TypeScript compilation passes without warnings
- [ ] Build completes successfully
- [ ] No missing component imports

## Automated Testing Setup (Future)

### Recommended Framework: Vitest + React Testing Library

Install dependencies:
```bash
npm install --save-dev vitest @testing-library/react @testing-library/jest-dom @vitest/ui
npm install --save-dev @types/vitest jsdom
```

### Example Test File Structure

```typescript
import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { MainNav } from './components/MainNav';

describe('MainNav Component', () => {
  it('renders navigation items', () => {
    render(<MainNav currentPage="overview" onNavigate={vi.fn()} />);
    expect(screen.getByText('Overview')).toBeInTheDocument();
  });

  it('highlights current page', () => {
    render(<MainNav currentPage="validators" onNavigate={vi.fn()} />);
    const validatorsLink = screen.getByText('Validators');
    expect(validatorsLink).toHaveClass('text-blue-400');
  });
});
```

### Test Categories

1. **Unit Tests**: Individual components
   - MainNav navigation state
   - ValidatorControls filtering
   - LeaderboardAndMetrics sorting
   - AdminControls tab switching

2. **Integration Tests**: Multi-component workflows
   - Login → Overview → Navigate to Validators
   - Admin login → Validator Controls → Approve validator
   - Metrics export flow

3. **E2E Tests**: Full user journeys
   - Complete operator workflow
   - Complete admin workflow
   - Data persistence across sessions

## Performance Checklist

- [ ] Bundle size < 800 kB gzipped
- [ ] Page load time < 2 seconds on 4G
- [ ] No unnecessary re-renders
- [ ] Sidebar animation smooth (60 fps)
- [ ] Search/filter responsive (< 100ms)

## Browser Compatibility

- [ ] Chrome/Chromium (latest)
- [ ] Firefox (latest)
- [ ] Safari (latest)
- [ ] Mobile Safari (iOS)
- [ ] Chrome Mobile (Android)

## Accessibility Checklist

- [ ] All buttons keyboard accessible (Tab key)
- [ ] Tab order logical
- [ ] Color contrast meets WCAG AA standards
- [ ] Form labels associated with inputs
- [ ] Icons have descriptive aria-labels (where applicable)
- [ ] No keyboard traps

## Deployment Checklist

- [ ] All console errors resolved
- [ ] Build passes without warnings
- [ ] Environment variables configured
- [ ] API endpoints configured correctly
- [ ] localStorage keys don't conflict
- [ ] No hardcoded development URLs
