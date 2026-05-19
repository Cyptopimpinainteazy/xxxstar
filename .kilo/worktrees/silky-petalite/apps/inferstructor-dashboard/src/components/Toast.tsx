import { useCallback, useState, type ReactNode } from 'react';
import { X } from 'lucide-react';
import { TOAST_AUTO_DISMISS_MS } from '../constants';
import { ToastContext, type Toast, type ToastType } from './toast-context';

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<Toast[]>([]);

  const removeToast = useCallback((id: string) => {
    setToasts(prev => prev.filter(toast => toast.id !== id));
  }, []);

  const addToast = useCallback((message: string, type: ToastType = 'info') => {
    const id = Date.now().toString();
    const newToast: Toast = { id, message, type };

    setToasts(prev => [...prev, newToast]);

    window.setTimeout(() => {
      removeToast(id);
    }, TOAST_AUTO_DISMISS_MS);
  }, [removeToast]);

  return (
    <ToastContext.Provider value={{ toasts, addToast, removeToast }}>
      {children}
      <ToastContainer toasts={toasts} onRemove={removeToast} />
    </ToastContext.Provider>
  );
}

interface ToastContainerProps {
  toasts: Toast[];
  onRemove: (id: string) => void;
}

function ToastContainer({ toasts, onRemove }: ToastContainerProps) {
  const getStyles = (type: ToastType) => {
    switch (type) {
      case 'success':
        return 'bg-green-900/20 border-green-700 text-green-300';
      case 'error':
        return 'bg-red-900/20 border-red-700 text-red-300';
      case 'warning':
        return 'bg-yellow-900/20 border-yellow-700 text-yellow-300';
      case 'info':
      default:
        return 'bg-blue-900/20 border-blue-700 text-blue-300';
    }
  };

  return (
    <div className="fixed bottom-4 right-4 z-50 space-y-3" role="region" aria-live="polite" aria-label="Notifications">
      {toasts.map(toast => (
        <div
          key={toast.id}
          className={`flex items-center justify-between gap-3 px-4 py-3 rounded-lg border ${getStyles(
            toast.type
          )} animate-in slide-in-from-right-4 fade-in`}
          role="alert"
        >
          <p className="text-sm font-medium">{toast.message}</p>
          <button
            onClick={() => onRemove(toast.id)}
            className="flex-shrink-0 hover:opacity-70 transition-opacity"
            aria-label={`Dismiss ${toast.type} notification`}
          >
            <X className="w-4 h-4" />
          </button>
        </div>
      ))}
    </div>
  );
}
