import { AlertCircle } from 'lucide-react';

interface ErrorBannerProps {
  error: string | null;
  onDismiss: () => void;
}

export function ErrorBanner({ error, onDismiss }: ErrorBannerProps) {
  if (!error) return null;

  return (
    <div className="mb-6 p-4 bg-red-900/20 border border-red-700 rounded-lg flex items-start gap-3" role="alert" aria-live="assertive">
      <AlertCircle className="w-5 h-5 text-red-400 flex-shrink-0 mt-0.5" aria-hidden="true" />
      <div className="flex-1">
        <p className="text-red-300 font-medium">Error loading stats</p>
        <p className="text-red-200 text-sm">{error}</p>
      </div>
      <button
        onClick={onDismiss}
        className="text-red-400 hover:text-red-300 text-sm font-medium"
        aria-label="Dismiss error"
      >
        Dismiss
      </button>
    </div>
  );
}
