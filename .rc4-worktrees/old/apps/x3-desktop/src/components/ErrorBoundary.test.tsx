/**
 * Tests for ErrorBoundary component
 */

import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { ErrorBoundary } from './ErrorBoundary';

describe('ErrorBoundary Component', () => {
  it('should render children when no error occurs', () => {
    render(
      <ErrorBoundary>
        <div>Test content</div>
      </ErrorBoundary>
    );

    expect(screen.getByText('Test content')).toBeInTheDocument();
  });

  it('should render ErrorBoundary wrapper without errors', () => {
    const { container } = render(
      <ErrorBoundary componentName="TestComponent">
        <div>Wrapped content</div>
      </ErrorBoundary>
    );

    expect(container).toBeInTheDocument();
    expect(screen.getByText('Wrapped content')).toBeInTheDocument();
  });

  it('should accept custom fallback prop', () => {
    const { container } = render(
      <ErrorBoundary fallback={<div>Custom error</div>}>
        <div>Normal content</div>
      </ErrorBoundary>
    );

    expect(container).toBeInTheDocument();
    expect(screen.getByText('Normal content')).toBeInTheDocument();
  });

  it('should accept onError callback prop', () => {
    const onError = vi.fn();

    const { container } = render(
      <ErrorBoundary onError={onError}>
        <div>Content</div>
      </ErrorBoundary>
    );

    expect(container).toBeInTheDocument();
    // onError should not be called when there's no error
    expect(onError).not.toHaveBeenCalled();
  });

  it('should render with all props', () => {
    const onError = vi.fn();
    const { container } = render(
      <ErrorBoundary 
        componentName="TestComponent"
        onError={onError}
        fallback={<div>Error UI</div>}
      >
        <div>Main content</div>
      </ErrorBoundary>
    );

    expect(container).toBeInTheDocument();
    expect(screen.getByText('Main content')).toBeInTheDocument();
  });

  it('should have proper styling setup', () => {
    const { container } = render(
      <ErrorBoundary>
        <div>Test</div>
      </ErrorBoundary>
    );

    // Verify the wrapper renders without styling issues
    expect(container.firstChild).toBeInTheDocument();
  });
});
