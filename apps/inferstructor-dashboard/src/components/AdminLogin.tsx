import { useState, type FormEvent } from 'react';
import { Shield, Lock, AlertCircle, Loader2 } from 'lucide-react';
import { api } from '../api';
import { validatePassword } from '../utils/validation';

interface AdminLoginProps {
  onLoginSuccess: () => void;
  onBack: () => void;
}

export function AdminLogin({ onLoginSuccess, onBack }: AdminLoginProps) {
  const [password, setPassword] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [validationError, setValidationError] = useState<string | null>(null);

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    
    const trimmedPassword = password.trim();
    if (!trimmedPassword) {
      setValidationError('Password is required');
      return;
    }

    const passwordValidation = validatePassword(trimmedPassword);
    if (!passwordValidation.valid) {
      setValidationError(passwordValidation.error || 'Invalid password');
      return;
    }

    setLoading(true);
    setError(null);
    setValidationError(null);

    try {
      await api.adminLogin(trimmedPassword);
      onLoginSuccess();
    } catch {
      setError('Invalid admin password');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="min-h-screen flex items-center justify-center p-6">
      <div className="w-full max-w-md">
        {/* Header */}
        <div className="text-center mb-8">
          <div className="inline-flex items-center justify-center w-16 h-16 rounded-full bg-red-900/30 border border-red-700/50 mb-4">
            <Shield className="w-8 h-8 text-red-400" />
          </div>
          <h1 className="text-3xl font-bold text-white mb-2">Admin Access</h1>
          <p className="text-gray-400 text-sm">Restricted area — authorized personnel only</p>
        </div>

        {/* Login Form */}
        <form onSubmit={handleSubmit} className="card">
          {error && (
            <div className="flex items-center gap-2 p-3 mb-4 bg-red-900/30 border border-red-700/50 rounded-lg text-red-300 text-sm">
              <AlertCircle className="w-4 h-4 flex-shrink-0" />
              <span>{error}</span>
            </div>
          )}

           <div className="mb-6">
             <label className="block text-sm font-medium text-gray-300 mb-2">
               Admin Password
             </label>
             <div className="relative">
               <Lock className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-500" />
               <input
                 type="password"
                 value={password}
                 onChange={(e) => setPassword(e.target.value)}
                 className={`w-full pl-10 pr-4 py-3 bg-gray-800/50 border rounded-lg text-white placeholder-gray-500 focus:outline-none focus:ring-1 transition-colors ${
                   validationError
                     ? 'border-red-500 bg-red-500/5 focus:border-red-500 focus:ring-red-500/50'
                     : 'border-gray-600 focus:border-red-500 focus:ring-red-500/50'
                 }`}
                 placeholder="Enter admin password"
                 autoFocus
                 autoComplete="current-password"
               />
             </div>
             {validationError && (
               <p className="text-red-400 text-sm mt-1 flex items-center gap-1">
                 <AlertCircle className="w-4 h-4" />
                 {validationError}
               </p>
             )}
           </div>

          <button
            type="submit"
            disabled={loading || !password.trim()}
            className="w-full py-3 bg-red-600 hover:bg-red-500 disabled:bg-gray-700 disabled:cursor-not-allowed text-white font-semibold rounded-lg transition-colors flex items-center justify-center gap-2"
          >
            {loading ? (
              <>
                <Loader2 className="w-4 h-4 animate-spin" />
                Authenticating...
              </>
            ) : (
              <>
                <Shield className="w-4 h-4" />
                Access Admin Panel
              </>
            )}
          </button>
        </form>

        <div className="text-center mt-6">
          <button
            onClick={onBack}
            className="text-gray-400 hover:text-white text-sm transition-colors"
          >
            ← Back to Dashboard
          </button>
        </div>
      </div>
    </div>
  );
}
