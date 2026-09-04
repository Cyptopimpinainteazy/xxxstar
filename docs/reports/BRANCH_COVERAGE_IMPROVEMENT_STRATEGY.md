# API-Mocked Test Variants Strategy

**Status**: READY FOR IMPLEMENTATION  
**Target**: 95% branch coverage on TestnetReadinessTile, AlertsPanel, CiStatusTile, MediaProductionPanel  
**Current Coverage**: 48% branches overall (63.33% when measured with extended tests)

## Why Standard Mocking Failed

The components as currently designed have a limitation: **all API data is hardcoded in useEffect**, not passed through props. This means:

```tsx
// Current pattern (can't test different responses):
useEffect(() => {
  const mockStatus: TestnetStatus = {
    networkHealth: 98,  // ← Always 98, can't test 70% or 45% branches
    nodesOnline: 23,
    totalNodes: 24,
    // ...
  };
  setStatus(mockStatus);
}, [rpcUrl]);
```

**Solution**: Refactor components to accept data as props or use a data provider pattern.

---

## Branch Coverage Gaps Analysis

### TestnetReadinessTile (40% branches)
**Uncovered branches**:
- Line 47 (getHealthColor): `if (status.networkHealth >= 95)` - always true with mock data
- Lines 60-61 (getHealthStatus): `>= 80` and `>= 60` branches untested
- Lines 66-68: Conditional renders (error, loading states) not all executed

**To fix**: Need to be able to mock different networkHealth values (98%, 85%, 70%, 45%)

### AlertsPanel (43.75% branches)
**Uncovered branches**:
- Line 41 (getSeverityColor switch): Only 'info' and 'warning' tested, missing 'critical' and 'error' branches
- Lines 61-63: getSeverityIcon switch cases
- Line 69: Different alert array states (empty, loading, populated)
- Lines 76-78: Alert rendering conditionals

**To fix**: Need to mock alerts with different severity levels

### CiStatusTile (50% branches)
**Uncovered branches**:
- Line 32: Error catch branch
- Lines 47-49: getStatusColor switch cases ('failed', 'pending', 'unknown')
- Lines 60-62: getStatusIcon switch cases
- PR conditional rendering branch

**To fix**: Need to mock different CI status responses

### MediaProductionPanel (50% branches)
**Uncovered branches**:
- Line 57: Error state path
- Line 69: Empty sessions array
- Status badge switches (completed, scheduled, failed paths)
- Progress bar width rendering

**To fix**: Need to mock different session statuses and empty/error states

---

## Implementation Path (3 Options)

### Option 1: Inject Data via Props (RECOMMENDED)

**Refactor pattern**:
```tsx
interface TestnetReadinessTileProps {
  rpcUrl: string;
  initialData?: TestnetStatus;  // ← For testing
}

export const TestnetReadinessTile: React.FC<TestnetReadinessTileProps> = ({
  rpcUrl,
  initialData,
}) => {
  const [status, setStatus] = useState<TestnetStatus>(
    initialData || { /* defaults */ }
  );
  // ...
};
```

**Test pattern**:
```tsx
it('should display "Excellent" for health >= 95%', () => {
  render(
    <TestnetReadinessTile
      rpcUrl="http://localhost:9944"
      initialData={{ networkHealth: 98 }}
    />
  );
  expect(screen.getByText('Excellent')).toBeInTheDocument();
});

it('should display "Good" for health >= 80%', () => {
  render(
    <TestnetReadinessTile
      rpcUrl="http://localhost:9944"
      initialData={{ networkHealth: 85 }}
    />
  );
  expect(screen.getByText('Good')).toBeInTheDocument();
});

it('should display "Poor" for health < 60%', () => {
  render(
    <TestnetReadinessTile
      rpcUrl="http://localhost:9944"
      initialData={{ networkHealth: 45 }}
    />
  );
  expect(screen.getByText('Poor')).toBeInTheDocument();
});
```

**Effort**: 2-3 hours for all 4 components  
**Coverage gain**: +30% branches (48% → 78%+)

---

### Option 2: Mock Module with jest.mock()

**Create mock API module**:
```tsx
// src/api/mock.ts
export const createTestnetStore = (initialData?: Partial<TestnetStatus>) => {
  return {
    getStatus: async () => ({
      networkHealth: 98,
      ...initialData,
    }),
  };
};

// Tests
jest.mock('../api', () => ({
  fetchStatus: jest.fn()
    .mockResolvedValueOnce({ networkHealth: 98 })
    .mockResolvedValueOnce({ networkHealth: 85 })
    .mockResolvedValueOnce({ networkHealth: 45 })
    .mockRejectedValueOnce(new Error('API failed')),
}));
```

**Effort**: 3-4 hours (requires extracting API calls from components first)  
**Coverage gain**: +30% branches with proper mock sequencing

---

### Option 3: Context Provider for Test Data

**Create test context**:
```tsx
const TestDataContext = React.createContext<{
  testnetStatus?: TestnetStatus;
  alerts?: Alert[];
  ciStatus?: CiStatus;
}>({});

// Wrapper for tests
const TestWrapper = ({ children, data }) => (
  <TestDataContext.Provider value={data}>
    {children}
  </TestDataContext.Provider>
);

// Component uses context in tests, real API in production
```

**Effort**: 2-3 hours (adds abstraction layer)  
**Coverage gain**: +30% branches with clean separation

---

## Quick Coverage Win Strategy

For **immediate 5-8% coverage improvement** without component refactoring:

1. **Fix error branches** (existing components can be error-tested):
   ```tsx
   it('should handle API errors gracefully', async () => {
     // Mock fetch to reject
     global.fetch = jest.fn().mockRejectedValueOnce(new Error('Network error'));
     
     render(<TestnetReadinessTile rpcUrl="http://localhost:9944" />);
     
     await waitFor(() => {
       // Error state would render
     });
   });
   ```

2. **Test loading states** (already present in components):
   ```tsx
   it('should display loading spinner', () => {
     render(<TestnetReadinessTile rpcUrl="http://localhost:9944" />);
     // Loading is briefly shown before mock data loads
     expect(screen.getByText(/Loading/i)).toBeInTheDocument();
   });
   ```

3. **Test prop combinations** (CiStatusTile already supports):
   ```tsx
   it('should display PR number when provided', () => {
     render(<CiStatusTile rpcUrl="url" branch="main" pr={123} />);
     expect(screen.getByText('PR #123')).toBeInTheDocument();
   });
   ```

---

## Recommended Approach

**Phase 1 (1-2 hours)**: Implement Quick Wins (error handling, prop variations)  
**Phase 2 (2-3 hours)**: Refactor to inject data via props  
**Phase 3 (ensure stable)**: Run full coverage suite and verify 95% branch coverage

### Concrete Test Examples

Once components are refactored to accept initialData props:

```tsx
// ✅ Full branch coverage for TestnetReadinessTile
describe('TestnetReadinessTile - Branch Coverage', () => {
  it('should display excellent status for 98% health', () => {
    render(<TestnetReadinessTile rpcUrl="url" initialData={{ networkHealth: 98 }} />);
    expect(screen.getByText('Excellent')).toBeInTheDocument();
    expect(screen.getByText('98%')).toBeInTheDocument();
  });

  it('should display good status for 85% health', () => {
    render(<TestnetReadinessTile rpcUrl="url" initialData={{ networkHealth: 85 }} />);
    expect(screen.getByText('Good')).toBeInTheDocument();
  });

  it('should display fair status for 70% health', () => {
    render(<TestnetReadinessTile rpcUrl="url" initialData={{ networkHealth: 70 }} />);
    expect(screen.getByText('Fair')).toBeInTheDocument();
  });

  it('should display poor status for 45% health', () => {
    render(<TestnetReadinessTile rpcUrl="url" initialData={{ networkHealth: 45 }} />);
    expect(screen.getByText('Poor')).toBeInTheDocument();
  });

  it('should display error state with error message', () => {
    render(<TestnetReadinessTile rpcUrl="url" initialError="Network failed" />);
    expect(screen.getByText(/Network failed/)).toBeInTheDocument();
  });

  it('should display loading state', () => {
    render(<TestnetReadinessTile rpcUrl="url" isLoading={true} />);
    expect(screen.getByText('Loading...')).toBeInTheDocument();
  });
});
```

---

## File Statistics

| Component | Branches | Lines | Target | Gap |
|-----------|----------|-------|--------|-----|
| TestnetReadinessTile | 40% | 72.97% | 95% | 55% |
| AlertsPanel | 43.75% | 76.66% | 95% | 51.25% |
| CiStatusTile | 50% | 80% | 95% | 45% |
| MediaProductionPanel | 50% | 91.66% | 95% | 45% |
| **Overall** | **48%** | **66.79%** | **95%** | **47%** |

---

## Next Steps (Post-Merge)

1. **Create feature branch**: `feat/test-branch-coverage`
2. **Implement Phase 1**: Quick wins with current components
3. **Implement Phase 2**: Refactor with initialData props
4. **Run coverage verification**: `npm run test:coverage`
5. **Target metrics**: 95% branches across all 4 components
6. **Timeline**: 3-4 hours estimated

---

## Success Criteria

✅ Branch coverage: 95%+  
✅ Component refactoring complete  
✅ All 4 components tested with mocked data  
✅ Test suite stable (no new failures)  
✅ PR merged with comprehensive test additions

