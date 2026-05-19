import { useState } from 'react';
import { api } from '../api';
import { Lock, Key, LogIn, ArrowLeft, AlertCircle } from 'lucide-react';
import { validateApiKey, validateApiSecret } from '../utils/validation';

interface LoginPageProps {
  onLoginSuccess: () => void;
  onBackToRegister: () => void;
}

export function LoginPage({ onLoginSuccess, onBackToRegister }: LoginPageProps) {
  const [apiKey, setApiKey] = useState('');
  const [apiSecret, setApiSecret] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [validationErrors, setValidationErrors] = useState<{ apiKey?: string; apiSecret?: string }>({});

  const validateForm = (): boolean => {
    const errors: { apiKey?: string; apiSecret?: string } = {};
    
    const apiKeyValidation = validateApiKey(apiKey);
    if (!apiKeyValidation.valid) {
      errors.apiKey = apiKeyValidation.error;
    }
    
    const apiSecretValidation = validateApiSecret(apiSecret);
    if (!apiSecretValidation.valid) {
      errors.apiSecret = apiSecretValidation.error;
    }
    
    setValidationErrors(errors);
    return Object.keys(errors).length === 0;
  };

  const handleLogin = async (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!validateForm()) {
      return;
    }

    setLoading(true);
    setError('');

    try {
      await api.login(apiKey, apiSecret);
      onLoginSuccess();
    } catch (err: any) {
      setError(err.message || 'Login failed. Please check your credentials.');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="min-h-screen flex items-center justify-center p-6">
      <div className="max-w-md w-full">
        <button
          onClick={onBackToRegister}
          className="flex items-center gap-2 text-gray-400 hover:text-white mb-6 transition-colors"
        >
          <ArrowLeft className="w-4 h-4" />
          Back to Registration
        </button>

        <div className="card">
          <div className="text-center mb-6">
            <div className="inline-flex items-center justify-center w-16 h-16 bg-blue-500/20 rounded-full mb-4">
              <Lock className="w-8 h-8 text-blue-400" />
            </div>
            <h2 className="text-2xl font-bold text-white mb-2">Welcome Back</h2>
            <p className="text-gray-400">Log in to your Inferstructor dashboard</p>
          </div>

          {error && (
            <div className="bg-red-900/20 border border-red-700/50 rounded-lg p-4 mb-6">
              <p className="text-red-400 text-sm">{error}</p>
            </div>
          )}

           <form onSubmit={handleLogin} className="space-y-4">
             <div>
               <label className="block text-sm font-medium text-gray-300 mb-2">
                 <Key className="w-4 h-4 inline mr-2" />
                 API Key
               </label>
               <input
                 type="text"
                 value={apiKey}
                 onChange={(e) => setApiKey(e.target.value)}
                 className={`input font-mono text-sm ${validationErrors.apiKey ? 'border-red-500 bg-red-500/5' : ''}`}
                 placeholder="infra_xxxxxxxxxxxxx"
                 required
               />
               {validationErrors.apiKey && (
                 <p className="text-red-400 text-sm mt-1 flex items-center gap-1">
                   <AlertCircle className="w-4 h-4" />
                   {validationErrors.apiKey}
                 </p>
               )}
             </div>

             <div>
               <label className="block text-sm font-medium text-gray-300 mb-2">
                 <Lock className="w-4 h-4 inline mr-2" />
                 API Secret
               </label>
               <input
                 type="password"
                 value={apiSecret}
                 onChange={(e) => setApiSecret(e.target.value)}
                 className={`input font-mono text-sm ${validationErrors.apiSecret ? 'border-red-500 bg-red-500/5' : ''}`}
                 placeholder="••••••••••••••••"
                 required
               />
               {validationErrors.apiSecret && (
                 <p className="text-red-400 text-sm mt-1 flex items-center gap-1">
                   <AlertCircle className="w-4 h-4" />
                   {validationErrors.apiSecret}
                 </p>
               )}
             </div>

            <button type="submit" disabled={loading} className="btn-primary w-full flex items-center justify-center gap-2">
              {loading ? (
                'Logging in...'
              ) : (
                <>
                  <LogIn className="w-4 h-4" />
                  Log In
                </>
              )}
            </button>
          </form>

          <div className="mt-6 p-4 bg-gray-900/50 rounded-lg">
            <p className="text-xs text-gray-400">
              💡 <strong className="text-gray-300">Tip:</strong> Your API key and secret were provided during registration.
              If you've lost them, you'll need to register a new validator.
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
