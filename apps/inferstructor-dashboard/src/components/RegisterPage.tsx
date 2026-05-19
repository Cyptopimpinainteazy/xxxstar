import { useState } from 'react';
import { api } from '../api';
import { Rocket, Mail, Server, Shield, ArrowRight, Sparkles, AlertCircle } from 'lucide-react';
import { validateEmail, validateChain, validateSlaTier } from '../utils/validation';

interface RegisterPageProps {
  onRegisterSuccess: () => void;
}

export function RegisterPage({ onRegisterSuccess }: RegisterPageProps) {
  const [chain, setChain] = useState('solana');
  const [email, setEmail] = useState('');
  const [slaTier, setSlaTier] = useState('pro');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [credentials, setCredentials] = useState<any>(null);
  const [validationErrors, setValidationErrors] = useState<{ chain?: string; email?: string; slaTier?: string }>({});

  const validateForm = (): boolean => {
    const errors: { chain?: string; email?: string; slaTier?: string } = {};

    const chainValidation = validateChain(chain);
    if (!chainValidation.valid) {
      errors.chain = chainValidation.error;
    }

    const emailValidation = validateEmail(email);
    if (!emailValidation.valid) {
      errors.email = emailValidation.error;
    }

    const slaTierValidation = validateSlaTier(slaTier);
    if (!slaTierValidation.valid) {
      errors.slaTier = slaTierValidation.error;
    }

    setValidationErrors(errors);
    return Object.keys(errors).length === 0;
  };

  const handleRegister = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!validateForm()) {
      return;
    }

    setLoading(true);
    setError('');

    try {
      const creds = await api.register(chain, email, slaTier);
      setCredentials(creds);
    } catch (err: any) {
      setError(err.message || 'Registration failed. Please try again.');
    } finally {
      setLoading(false);
    }
  };

  if (credentials) {
    return (
      <div className="min-h-screen flex items-center justify-center p-6">
        <div className="card max-w-2xl w-full">
          <div className="text-center mb-6">
            <div className="inline-flex items-center justify-center w-16 h-16 bg-green-500/20 rounded-full mb-4">
              <Sparkles className="w-8 h-8 text-green-400" />
            </div>
            <h2 className="text-3xl font-bold text-white mb-2">Registration Successful!</h2>
            <p className="text-gray-400">Save these credentials securely. They won't be shown again.</p>
          </div>

          <div className="space-y-4">
            <div className="bg-gray-900/50 rounded-lg p-4">
              <label className="text-sm text-gray-400">Validator ID</label>
              <div className="flex items-center justify-between mt-1">
                <code className="text-white font-mono text-sm">{credentials.validator_id}</code>
                <button
                  onClick={() => navigator.clipboard.writeText(credentials.validator_id)}
                  className="text-blue-400 hover:text-blue-300 text-sm"
                >
                  Copy
                </button>
              </div>
            </div>

            <div className="bg-gray-900/50 rounded-lg p-4">
              <label className="text-sm text-gray-400">API Key</label>
              <div className="flex items-center justify-between mt-1">
                <code className="text-white font-mono text-sm truncate mr-4">{credentials.api_key}</code>
                <button
                  onClick={() => navigator.clipboard.writeText(credentials.api_key)}
                  className="text-blue-400 hover:text-blue-300 text-sm flex-shrink-0"
                >
                  Copy
                </button>
              </div>
            </div>

            {credentials.api_secret && (
              <div className="bg-red-900/20 border border-red-700/50 rounded-lg p-4">
                <label className="text-sm text-red-400 font-semibold">API Secret (Save Now!)</label>
                <div className="flex items-center justify-between mt-1">
                  <code className="text-white font-mono text-sm truncate mr-4">{credentials.api_secret}</code>
                  <button
                    onClick={() => navigator.clipboard.writeText(credentials.api_secret)}
                    className="text-red-400 hover:text-red-300 text-sm flex-shrink-0"
                  >
                    Copy
                  </button>
                </div>
                <p className="text-xs text-red-400 mt-2">⚠️ This secret is only shown once. Store it securely!</p>
              </div>
            )}

            <div className="grid grid-cols-2 gap-4">
              <div className="bg-gray-900/50 rounded-lg p-4">
                <label className="text-sm text-gray-400">SLA Tier</label>
                <p className="text-white font-semibold capitalize">{credentials.sla_tier}</p>
              </div>
              <div className="bg-gray-900/50 rounded-lg p-4">
                <label className="text-sm text-gray-400">Max TPS</label>
                <p className="text-white font-semibold">{credentials.max_tps.toLocaleString()}</p>
              </div>
            </div>
          </div>

          <button
            onClick={onRegisterSuccess}
            className="btn-primary w-full mt-6 flex items-center justify-center gap-2"
          >
            Go to Dashboard
            <ArrowRight className="w-4 h-4" />
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen flex items-center justify-center p-6">
      <div className="max-w-6xl w-full">
        <div className="text-center mb-12">
          <div className="inline-flex items-center justify-center gap-3 mb-4">
            <Rocket className="w-12 h-12 text-blue-400" />
            <h1 className="text-5xl font-bold bg-gradient-to-r from-blue-400 to-purple-400 bg-clip-text text-transparent">
              Inferstructor
            </h1>
          </div>
          <p className="text-xl text-gray-300 mb-2">300× Faster Blockchain Validation</p>
          <p className="text-gray-400">GPU-Accelerated Superhighway for Any Chain</p>
        </div>

        <div className="grid md:grid-cols-2 gap-8">
          {/* Registration Form */}
          <div className="card">
            <h2 className="text-2xl font-bold text-white mb-6">Register Your Validator</h2>
            
            {error && (
              <div className="bg-red-900/20 border border-red-700/50 rounded-lg p-4 mb-6 flex items-center gap-2">
                <AlertCircle className="w-4 h-4 text-red-400 flex-shrink-0" />
                <p className="text-red-400 text-sm">{error}</p>
              </div>
            )}
            
            <form onSubmit={handleRegister} className="space-y-4">
              <div>
                <label className="block text-sm font-medium text-gray-300 mb-2">
                  <Server className="w-4 h-4 inline mr-2" />
                  Blockchain
                </label>
                <select
                  value={chain}
                  onChange={(e) => setChain(e.target.value)}
                  className={`input ${validationErrors.chain ? 'border-red-500 bg-red-500/5' : ''}`}
                  required
                >
                  <option value="solana">Solana</option>
                  <option value="ethereum">Ethereum</option>
                  <option value="arbitrum">Arbitrum</option>
                  <option value="polygon">Polygon</option>
                  <option value="avalanche">Avalanche</option>
                  <option value="other">Other</option>
                </select>
                {validationErrors.chain && (
                  <p className="text-red-400 text-sm mt-1 flex items-center gap-1">
                    <AlertCircle className="w-4 h-4" />
                    {validationErrors.chain}
                  </p>
                )}
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-300 mb-2">
                  <Mail className="w-4 h-4 inline mr-2" />
                  Email
                </label>
                <input
                  type="email"
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                  className={`input ${validationErrors.email ? 'border-red-500 bg-red-500/5' : ''}`}
                  placeholder="validator@example.com"
                  required
                />
                {validationErrors.email && (
                  <p className="text-red-400 text-sm mt-1 flex items-center gap-1">
                    <AlertCircle className="w-4 h-4" />
                    {validationErrors.email}
                  </p>
                )}
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-300 mb-2">
                  <Shield className="w-4 h-4 inline mr-2" />
                  SLA Tier
                </label>
                <div className="space-y-2">
                  {['basic', 'pro', 'enterprise'].map((tier) => (
                    <label key={tier} className={`
                      flex items-center justify-between p-4 rounded-lg border-2 cursor-pointer transition-all
                      ${slaTier === tier
                        ? 'border-blue-500 bg-blue-500/10'
                        : 'border-gray-700 bg-gray-800/30 hover:border-gray-600'
                      }
                    `}>
                      <div className="flex items-center gap-3">
                        <input
                          type="radio"
                          name="sla_tier"
                          value={tier}
                          checked={slaTier === tier}
                          onChange={(e) => setSlaTier(e.target.value)}
                          className="w-4 h-4 text-blue-500"
                        />
                        <div>
                          <p className="font-semibold text-white capitalize">{tier}</p>
                          <p className="text-xs text-gray-400">
                            {tier === 'basic' && '100K TPS • $10/M TX'}
                            {tier === 'pro' && '1M TPS • $50/M TX'}
                            {tier === 'enterprise' && 'Unlimited • $200/M TX'}
                          </p>
                        </div>
                      </div>
                    </label>
                  ))}
                </div>
                {validationErrors.slaTier && (
                  <p className="text-red-400 text-sm mt-2 flex items-center gap-1">
                    <AlertCircle className="w-4 h-4" />
                    {validationErrors.slaTier}
                  </p>
                )}
              </div>

              <button type="submit" disabled={loading} className="btn-primary w-full">
                {loading ? 'Registering...' : 'Register Now'}
              </button>
            </form>
          </div>

          {/* Features */}
          <div className="space-y-6">
            <div className="card">
              <h3 className="text-xl font-bold text-white mb-4">🚀 Why Inferstructor?</h3>
              <ul className="space-y-3 text-gray-300">
                <li className="flex items-start gap-3">
                  <span className="text-green-400 mt-1">✓</span>
                  <span><strong className="text-white">300× Speed:</strong> 19.5M TPS vs Solana's 65K</span>
                </li>
                <li className="flex items-start gap-3">
                  <span className="text-green-400 mt-1">✓</span>
                  <span><strong className="text-white">Sub-ms Latency:</strong> GPU acceleration with &lt;500μs response</span>
                </li>
                <li className="flex items-start gap-3">
                  <span className="text-green-400 mt-1">✓</span>
                  <span><strong className="text-white">Multi-Lane:</strong> Primary/Shadow/Tertiary failover</span>
                </li>
                <li className="flex items-start gap-3">
                  <span className="text-green-400 mt-1">✓</span>
                  <span><strong className="text-white">Zero Downtime:</strong> &lt;3ms failover switching</span>
                </li>
                <li className="flex items-start gap-3">
                  <span className="text-green-400 mt-1">✓</span>
                  <span><strong className="text-white">Pay-Per-Use:</strong> Only pay for what you process</span>
                </li>
              </ul>
            </div>

            <div className="card bg-gradient-to-br from-blue-900/20 to-purple-900/20 border-blue-700/50">
              <h3 className="text-xl font-bold text-white mb-2">🎯 Get Started in 3 Steps</h3>
              <div className="space-y-2 text-gray-300">
                <p><span className="text-blue-400 font-bold">1.</span> Register your validator</p>
                <p><span className="text-blue-400 font-bold">2.</span> Get API credentials</p>
                <p><span className="text-blue-400 font-bold">3.</span> Start accelerating</p>
              </div>
              <p className="text-sm text-gray-400 mt-4">⚡ Setup time: &lt;5 minutes</p>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
