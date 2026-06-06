# Error Handling System

Comprehensive error handling and recovery system for X3 Desktop monitoring components.

## Features

✅ **Error Classification** - Automatically categorize errors (network, timeout, Tauri unavailable, etc.)
✅ **Retry Logic** - Exponential backoff retry mechanism with configurable attempts
✅ **Error Boundaries** - React Error Boundary component to catch component crashes
✅ **Network Detection** - Detect offline status and provide appropriate messaging
✅ **Tauri Fallback** - Gracefully handle when Tauri IPC is unavailable
✅ **User-Friendly Messages** - Context-aware error messages for end users
✅ **Error Logging** - Structured error logging for debugging and monitoring
✅ **Custom Hooks** - `useAsync`, `useNetworkStatus`, `useTauriAvailable` for common patterns

## Components

### ErrorBoundary

React Error Boundary component that catches component crashes.

```tsx
import { ErrorBoundary } from '@/components/ErrorBoundary';

<ErrorBoundary componentName="MyComponent" onError={handleError}>
  <MyComponent />
</ErrorBoundary>
```

**Props:**
- `children` - Component to wrap
- `fallback` - Custom fallback UI (optional)
- `onError` - Callback when error occurs (optional)
- `componentName` - Name for error tracking (optional)

**Features:**
- Displays user-friendly error messages
- "Try Again" and "Reload App" buttons
- Debug information in development mode
- Auto-reset after 30 seconds if rare
- Tracks multiple error occurrences

### Error Handlers

Located in `src/utils/errorHandler.ts`

#### `classifyError(error: any): AppError`

Classifies errors into categories:
- `TAURI_NOT_AVAILABLE` - Tauri IPC unavailable
- `NETWORK_ERROR` - Network/fetch errors
- `TIMEOUT` - Request timeout
- `INVALID_DATA` - Parse/data errors
- `OFFLINE` - Device offline
- `UNKNOWN` - Unknown error

```tsx
const appError = classifyError(error);
console.log(appError.code);      // Error category
console.log(appError.message);   // User-friendly message
```

#### `withRetry<T>(fn: () => Promise<T>, options): Promise<T>`

Executes async function with exponential backoff retry.

```tsx
const data = await withRetry(
  () => invoke<Data>('fetch_data'),
  {
    maxAttempts: 3,
    delayMs: 1000,
    backoffMultiplier: 2,
    timeout: 10000,
  }
);
```

**Options:**
- `maxAttempts` - Number of retry attempts (default: 3)
- `delayMs` - Initial delay in ms (default: 1000)
- `backoffMultiplier` - Multiplier for exponential backoff (default: 2)
- `timeout` - Timeout per attempt in ms (default: 10000)

#### `logError(error: AppError, context?: string): void`

Logs classified error with context.

```tsx
const appError = classifyError(error);
logError(appError, 'SystemMetricsPanel');
```

#### `createErrorContext(componentName: string, info?): ErrorContext`

Creates structured error context with metadata.

```tsx
const context = createErrorContext('MyComponent', {
  userId: user.id,
  action: 'fetch_metrics',
});
```

## Hooks

### `useAsync<T>(asyncFn, options)`

Hook for safe async operations with retry and error handling.

```tsx
const { data, loading, error, retry, refetch } = useAsync(
  () => invoke<Data>('fetch_data'),
  {
    maxRetries: 3,
    autoFetch: true,
  }
);

if (error) return <ErrorUI message={error.message} onRetry={retry} />;
if (loading) return <LoadingUI />;
return <DataUI data={data} />;
```

### `useNetworkStatus(): boolean`

Hook to detect online/offline status.

```tsx
const isOnline = useNetworkStatus();

if (!isOnline) {
  return <OfflineMessage />;
}
```

### `useTauriAvailable(): boolean`

Hook to detect if Tauri is available.

```tsx
const tauriAvailable = useTauriAvailable();

if (!tauriAvailable) {
  return <TauriUnavailableMessage />;
}
```

## Usage Examples

### In Components

```tsx
import { useAsync } from '@/hooks/useAsync';
import { ErrorBoundary } from '@/components/ErrorBoundary';

export const MyComponent: React.FC = () => {
  const { data, loading, error, retry } = useAsync(
    () => invoke<Data>('fetch_data')
  );

  if (error) {
    return (
      <ErrorUI 
        message={error.message}
        retryCount={error.retryCount}
        onRetry={retry}
      />
    );
  }

  if (loading) return <LoadingUI />;
  return <DataUI data={data} />;
};

// Wrap the component
<ErrorBoundary componentName="MyComponent">
  <MyComponent />
</ErrorBoundary>
```

### Monitoring Panels

The `SystemMetricsPanel` and `IpfsStoragePanel` now include:

- **Retry Logic**: Automatic retry with exponential backoff
- **Error Messages**: Contextual, user-friendly error messages
- **Network Detection**: Special messaging when offline
- **Retry Counter**: Shows retry attempts to user
- **Max Retry Limit**: Prevents infinite retry loops

### Global Error Handling

The `MonitoringDashboard` wraps both panels with error boundaries:

```tsx
<ErrorBoundary componentName="SystemMetricsPanel" onError={handleError}>
  <SystemMetricsPanel />
</ErrorBoundary>

<ErrorBoundary componentName="IpfsStoragePanel" onError={handleError}>
  <IpfsStoragePanel />
</ErrorBoundary>
```

## Error Messages

### Common Errors Handled

| Error | Classification | User Message |
|-------|-----------------|--------------|
| Tauri not available | `TAURI_NOT_AVAILABLE` | "Desktop environment not initialized. Please restart the application." |
| Network failure | `NETWORK_ERROR` | "Network connection failed. Please check your connection." |
| Offline | `OFFLINE` | "You are offline. Please check your internet connection." |
| Request timeout | `TIMEOUT` | "Request took too long. Please try again." |
| Invalid data | `INVALID_DATA` | "Received invalid data from server. Please try again." |

## Error Logging Integration

The system supports integration with error tracking services:

```tsx
// In errorHandler.ts, modify logError:
export const logError = (error: AppError, context?: string): void => {
  const logEntry = { context, ...error };
  console.error('[AppError]', logEntry);
  
  // Send to Sentry, LogRocket, etc:
  trackError(logEntry);
};
```

## Best Practices

1. **Always wrap async operations** with `withRetry` or `useAsync`
2. **Use ErrorBoundary** at appropriate component levels
3. **Provide helpful error messages** to guide user recovery
4. **Log errors** with proper context for debugging
5. **Allow retries** but set reasonable limits
6. **Handle offline state** gracefully
7. **Test error paths** as thoroughly as happy paths

## Testing

Error handling includes comprehensive tests:

```bash
npm run test -- errorHandler.test.ts
npm run test -- ErrorBoundary.test.tsx
```

See test files for examples of testing error scenarios.

## Future Enhancements

- [ ] Error tracking service integration (Sentry, LogRocket)
- [ ] Persistent error history with timestamps
- [ ] User preference for error notification style
- [ ] Automated error reporting to backend
- [ ] Error analytics and trending
- [ ] Custom error recovery workflows per error type
