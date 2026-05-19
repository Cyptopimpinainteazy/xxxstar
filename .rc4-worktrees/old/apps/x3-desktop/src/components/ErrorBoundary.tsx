import React, { ReactNode, ReactElement } from 'react';
import { AppError, classifyError, logError, createErrorContext } from '../utils/errorHandler';

interface Props {
  children: ReactNode;
  fallback?: ReactNode;
  onError?: (error: AppError) => void;
  componentName?: string;
}

interface State {
  hasError: boolean;
  error: AppError | null;
  errorCount: number;
}

/**
 * Global Error Boundary Component
 * Catches React component errors and displays graceful error UI
 */
export class ErrorBoundary extends React.Component<Props, State> {
  private resetTimeout: ReturnType<typeof setTimeout> | null = null;

  constructor(props: Props) {
    super(props);
    this.state = {
      hasError: false,
      error: null,
      errorCount: 0,
    };
  }

  static getDerivedStateFromError(_error: Error): Partial<State> {
    return { hasError: true };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    const classifiedError = classifyError(error);
    const context = createErrorContext(this.props.componentName || 'Unknown', {
      errorInfo: errorInfo.componentStack,
    });

    classifiedError.context = context;

    this.setState((prevState) => ({
      error: classifiedError,
      errorCount: prevState.errorCount + 1,
    }));

    logError(classifiedError, this.props.componentName);
    this.props.onError?.(classifiedError);

    // Auto-reset after 30 seconds if error happens rarely
    if (this.state.errorCount < 3) {
      this.resetTimeout = setTimeout(() => {
        this.setState({ hasError: false, error: null });
      }, 30000);
    }
  }

  componentWillUnmount() {
    if (this.resetTimeout) {
      clearTimeout(this.resetTimeout);
    }
  }

  handleRetry = () => {
    this.setState({ hasError: false, error: null, errorCount: 0 });
  };

  render(): ReactNode {
    if (this.state.hasError && this.state.error) {
      return (
        this.props.fallback || (
          <div className="flex items-center justify-center w-full h-full min-h-screen bg-gray-950">
            <div className="w-full max-w-md p-6">
              <div className="bg-gray-900 rounded-lg border border-red-500/50 p-6">
                {/* Error Icon */}
                <div className="mb-4 text-center">
                  <div className="inline-flex items-center justify-center w-12 h-12 rounded-full bg-red-500/20">
                    <svg
                      className="w-6 h-6 text-red-500"
                      fill="none"
                      viewBox="0 0 24 24"
                      stroke="currentColor"
                    >
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M12 8v4m0 4v.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                      />
                    </svg>
                  </div>
                </div>

                {/* Error Title */}
                <h2 className="text-xl font-bold text-white mb-2 text-center">
                  Something went wrong
                </h2>

                {/* Error Message */}
                <p className="text-gray-300 text-sm text-center mb-4">
                  {this.state.error.message}
                </p>

                {/* Error Code */}
                <div className="bg-gray-800 rounded p-3 mb-4">
                  <p className="text-xs text-gray-400 font-mono mb-1">
                    Error Code:
                  </p>
                  <p className="text-sm text-gray-300 font-mono">
                    {this.state.error.code}
                  </p>
                </div>

                {/* Debug Info (only in development) */}
                {(import.meta as any).env.DEV && (
                  <details className="mb-4">
                    <summary className="text-xs text-gray-400 cursor-pointer hover:text-gray-300">
                      Debug Information
                    </summary>
                    <div className="mt-2 bg-gray-950 rounded p-2 text-xs text-gray-500 font-mono overflow-auto max-h-40">
                      <p className="mb-2">
                        <strong>Timestamp:</strong> {this.state.error.timestamp}
                      </p>
                      {this.state.error.context && (
                        <p>
                          <strong>Context:</strong>{' '}
                          {JSON.stringify(this.state.error.context, null, 2)}
                        </p>
                      )}
                      {this.state.error.originalError && (
                        <p>
                          <strong>Original:</strong>{' '}
                          {this.state.error.originalError.message}
                        </p>
                      )}
                    </div>
                  </details>
                )}

                {/* Action Buttons */}
                <div className="flex gap-3">
                  <button
                    onClick={this.handleRetry}
                    className="flex-1 px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white font-medium rounded transition-colors"
                  >
                    Try Again
                  </button>
                  <button
                    onClick={() => window.location.reload()}
                    className="flex-1 px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white font-medium rounded transition-colors"
                  >
                    Reload App
                  </button>
                </div>

                {/* Error Count Warning */}
                {this.state.errorCount > 3 && (
                  <p className="text-xs text-yellow-500 mt-4 text-center">
                    Multiple errors encountered ({this.state.errorCount}).
                    Please consider restarting the application.
                  </p>
                )}
              </div>
            </div>
          </div>
        )
      );
    }

    return this.props.children as ReactElement;
  }
}
