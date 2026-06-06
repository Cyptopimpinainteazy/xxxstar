'use client';

import { useState } from 'react';
import { Button, Badge } from '@/components/x3/UIComponents';
import {
  BarChart,
  Bar,
  PieChart,
  Pie,
  Cell,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from 'recharts';

const ACTIVE_LAUNCHES = [
  {
    id: 1,
    name: 'FutureAI Token',
    ticker: 'FAI',
    stage: 'Active IDO',
    progress: 67,
    raised: '$2.4M',
    goal: '$3.5M',
    days_left: 5,
    participants: 3847,
    price: '$0.15',
    image: '🤖',
  },
  {
    id: 2,
    name: 'GreenChain Eco',
    ticker: 'GREEN',
    stage: 'Active IDO',
    progress: 42,
    raised: '$1.2M',
    goal: '$2.8M',
    days_left: 12,
    participants: 2156,
    price: '$0.08',
    image: '🌱',
  },
  {
    id: 3,
    name: 'MetaVerse Pro',
    ticker: 'MVP',
    stage: 'Active IDO',
    progress: 85,
    raised: '$4.2M',
    goal: '$4.9M',
    days_left: 2,
    participants: 5421,
    price: '$0.22',
    image: '🚀',
  },
];

const TOKENOMICS_DATA = [
  { name: 'Team', value: 20, fill: '#ff6b6b' },
  { name: 'Community', value: 35, fill: '#00d4aa' },
  { name: 'IDO', value: 25, fill: '#ffa500' },
  { name: 'Liquidity', value: 20, fill: '#4ecdc4' },
];

const FUNDRAISING_MODELS = [
  {
    title: 'Traditional IDO',
    icon: '📊',
    description: 'Classic Initial DEX Offering model',
    features: [
      'Fixed price during IDO period',
      'Whitelist support for early backers',
      'Vesting schedules available',
      'Community voting on allocations',
    ],
  },
  {
    title: 'Fair Launch',
    icon: '⚖️',
    description: 'Equal opportunity for all participants',
    features: [
      'No pre-sale or whitelist',
      'Equal allocation per participant',
      'Transparent liquidity pool seeding',
      'Automatic dispersion at launch',
    ],
  },
  {
    title: 'Revenue Sharing',
    icon: '💰',
    description: 'Tokenholders earn protocol fees',
    features: [
      'Revenue share from trading fees (2-5%)',
      'Monthly dividend distributions',
      'Governance participation',
      'Staking rewards for holders',
    ],
  },
  {
    title: 'Hybrid Model',
    icon: '🔄',
    description: 'Combine multiple funding models',
    features: [
      'Initial token sale + revenue share',
      'Phased rollout over 6 months',
      'Customizable vesting periods',
      'Adaptive allocation model',
    ],
  },
];

const RAISED_CHART = Array.from({ length: 12 }, (_, i) => ({
  month: ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'][i],
  raised: 1200 + i * 180 + (i % 3) * 120,
}));

export default function LaunchpadPage() {
  const [selectedLaunch, setSelectedLaunch] = useState(ACTIVE_LAUNCHES[0]);
  const [showSubmitForm, setShowSubmitForm] = useState(false);
  const [selectedModel, setSelectedModel] = useState<number | null>(null);

  return (
    <div className="min-h-screen bg-gradient-to-br from-x3-dark via-x3-dark to-[#0f0f13] p-6 space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-4xl font-bold text-white">🚀 X3 Token Launchpad</h1>
          <p className="text-gray-400 mt-1">Launch Your Web3 Project | Next Generation Fundraising</p>
        </div>
        <Button
          onClick={() => setShowSubmitForm(!showSubmitForm)}
          className="px-6 py-3 bg-x3-orange hover:bg-orange-600 text-white font-bold rounded-lg"
        >
          🎯 Submit Project
        </Button>
      </div>

      {/* Stats Bar */}
      <div className="grid grid-cols-5 gap-4">
        <div className="bg-gradient-to-br from-x3-dark to-x3-dark-gray p-4 rounded-lg border border-x3-dark-gray">
          <div className="text-gray-400 text-xs font-bold">TOTAL RAISED</div>
          <div className="text-xl font-bold text-x3-orange mt-1">$127.3M</div>
          <div className="text-xs text-gray-500 mt-1">↑ 28% YoY</div>
        </div>
        <div className="bg-gradient-to-br from-x3-dark to-x3-dark-gray p-4 rounded-lg border border-x3-dark-gray">
          <div className="text-gray-400 text-xs font-bold">ACTIVE LAUNCHES</div>
          <div className="text-xl font-bold text-green-400 mt-1">3</div>
          <div className="text-xs text-gray-500 mt-1">Raising: $8.8M</div>
        </div>
        <div className="bg-gradient-to-br from-x3-dark to-x3-dark-gray p-4 rounded-lg border border-x3-dark-gray">
          <div className="text-gray-400 text-xs font-bold">PARTICIPANTS</div>
          <div className="text-xl font-bold text-cyan-400 mt-1">34,621</div>
          <div className="text-xs text-gray-500 mt-1">↑ 42% this month</div>
        </div>
        <div className="bg-gradient-to-br from-x3-dark to-x3-dark-gray p-4 rounded-lg border border-x3-dark-gray">
          <div className="text-gray-400 text-xs font-bold">AVG. RETURNS</div>
          <div className="text-xl font-bold text-green-400 mt-1">342%</div>
          <div className="text-xs text-gray-500 mt-1">Since platform launch</div>
        </div>
        <div className="bg-gradient-to-br from-x3-dark to-x3-dark-gray p-4 rounded-lg border border-x3-dark-gray">
          <div className="text-gray-400 text-xs font-bold">SUCCESSFUL TOKENS</div>
          <div className="text-xl font-bold text-x3-orange mt-1">127</div>
          <div className="text-xs text-gray-500 mt-1">Listed on major DEXs</div>
        </div>
      </div>

      {/* Active Launches */}
      <div>
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-2xl font-bold text-white">⏱️ Active IDO Sales</h2>
          <Badge variant="green">🟢 Live Now</Badge>
        </div>
        <div className="grid grid-cols-3 gap-4">
          {ACTIVE_LAUNCHES.map((launch) => (
            <div
              key={launch.id}
              onClick={() => setSelectedLaunch(launch)}
              className={`p-4 rounded-lg border cursor-pointer transition-all ${
                selectedLaunch.id === launch.id
                  ? 'bg-x3-orange border-x3-orange'
                  : 'bg-x3-dark border-x3-dark-gray hover:border-x3-orange'
              }`}
            >
              <div className="flex justify-between items-start mb-3">
                <div className="text-3xl">{launch.image}</div>
                <Badge>{launch.stage}</Badge>
              </div>
              <h3 className={`font-bold text-sm ${selectedLaunch.id === launch.id ? 'text-white' : 'text-gray-200'}`}>
                {launch.name}
              </h3>
              <p className="text-xs text-gray-400 mb-3">{launch.ticker}</p>

              {/* Progress Bar */}
              <div className="mb-3">
                <div className="flex justify-between text-xs mb-1">
                  <span className={selectedLaunch.id === launch.id ? 'text-white' : 'text-gray-300'}>
                    {launch.raised}
                  </span>
                  <span className="text-gray-500">{launch.progress}%</span>
                </div>
                <div className="h-2 bg-x3-dark-gray rounded-full overflow-hidden">
                  <div
                    className="h-full bg-gradient-to-r from-green-400 to-cyan-400"
                    style={{ width: `${launch.progress}%` }}
                  ></div>
                </div>
              </div>

              <div className="space-y-2">
                <div className="flex justify-between text-xs">
                  <span className="text-gray-500">Price:</span>
                  <span className={selectedLaunch.id === launch.id ? 'text-white font-bold' : 'text-gray-200'}>
                    {launch.price}
                  </span>
                </div>
                <div className="flex justify-between text-xs">
                  <span className="text-gray-500">Ends in:</span>
                  <span className="text-green-400 font-bold">{launch.days_left}d</span>
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Selected Launch Details */}
      <div className="grid grid-cols-3 gap-6">
        {/* Left - Info */}
        <div className="col-span-2 space-y-4">
          {/* Project Info */}
          <div className="bg-gradient-to-b from-x3-dark to-x3-dark-gray p-6 rounded-lg border border-x3-dark-gray">
            <div className="flex justify-between items-start mb-4">
              <div>
                <h2 className="text-2xl font-bold text-white flex items-center gap-2">
                  {selectedLaunch.image} {selectedLaunch.name}
                </h2>
                <p className="text-gray-400 mt-1">Ticker: {selectedLaunch.ticker}</p>
              </div>
              <Button className="px-6 py-3 bg-green-600 hover:bg-green-700 text-white font-bold rounded-lg">
                💳 Participate
              </Button>
            </div>

            <div className="grid grid-cols-3 gap-4 mb-4">
              <div>
                <div className="text-gray-400 text-xs mb-1">RAISED</div>
                <div className="text-xl font-bold text-x3-orange">{selectedLaunch.raised}</div>
              </div>
              <div>
                <div className="text-gray-400 text-xs mb-1">GOAL</div>
                <div className="text-xl font-bold text-white">{selectedLaunch.goal}</div>
              </div>
              <div>
                <div className="text-gray-400 text-xs mb-1">PARTICIPANTS</div>
                <div className="text-xl font-bold text-cyan-400">{selectedLaunch.participants.toLocaleString()}</div>
              </div>
            </div>

            {/* Main Progress */}
            <div className="mb-4">
              <div className="flex justify-between mb-2">
                <span className="text-gray-300">Progress to Goal</span>
                <span className="text-x3-orange font-bold">{selectedLaunch.progress}%</span>
              </div>
              <div className="h-3 bg-x3-dark-gray rounded-full overflow-hidden">
                <div
                  className="h-full bg-gradient-to-r from-green-400 via-cyan-400 to-blue-500"
                  style={{ width: `${selectedLaunch.progress}%` }}
                ></div>
              </div>
            </div>

            <p className="text-gray-400 text-sm">
              Join {selectedLaunch.participants.toLocaleString()} investors in this exciting project. {selectedLaunch.days_left} days
              remaining to participate at current price.
            </p>
          </div>

          {/* Tokenomics */}
          <div className="bg-gradient-to-b from-x3-dark to-x3-dark-gray p-6 rounded-lg border border-x3-dark-gray">
            <h3 className="text-lg font-bold text-white mb-4">📊 Token Distribution</h3>
            <div className="grid grid-cols-2 gap-4">
              <ResponsiveContainer width="100%" height={250}>
                <PieChart>
                  <Pie data={TOKENOMICS_DATA} cx="50%" cy="50%" outerRadius={80} dataKey="value">
                    {TOKENOMICS_DATA.map((entry, index) => (
                      <Cell key={`cell-${index}`} fill={entry.fill} />
                    ))}
                  </Pie>
                  <Tooltip />
                </PieChart>
              </ResponsiveContainer>

              <div className="space-y-3">
                {TOKENOMICS_DATA.map((item) => (
                  <div key={item.name} className="flex justify-between items-center">
                    <div className="flex items-center gap-2">
                      <div
                        className="w-3 h-3 rounded-full"
                        style={{ backgroundColor: item.fill }}
                      ></div>
                      <span className="text-gray-300">{item.name}</span>
                    </div>
                    <span className="font-bold text-white">{item.value}%</span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>

        {/* Right - Participation Info */}
        <div className="space-y-4">
          {/* Purchase Info */}
          <div className="bg-gradient-to-br from-green-900/20 to-transparent p-4 rounded-lg border border-green-600/30">
            <h3 className="font-bold text-green-400 mb-3">💰 Purchase Details</h3>
            <div className="space-y-2">
              <div className="flex justify-between text-sm">
                <span className="text-gray-300">Token Price:</span>
                <span className="font-bold text-white">{selectedLaunch.price}</span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-gray-300">Min. Amount:</span>
                <span className="font-bold text-white">$100</span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-gray-300">Max. Amount:</span>
                <span className="font-bold text-white">$50,000</span>
              </div>
              <div className="border-t border-green-600/30 pt-2 mt-2 flex justify-between text-sm">
                <span className="text-gray-300">You Get:</span>
                <span className="font-bold text-green-400">0 {selectedLaunch.ticker}</span>
              </div>
            </div>
          </div>

          {/* Sale Status */}
          <div className="bg-gradient-to-br from-x3-dark to-x3-dark-gray p-4 rounded-lg border border-x3-dark-gray">
            <h3 className="font-bold text-white mb-3">📅 Sale Status</h3>
            <div className="space-y-2">
              <div className="flex justify-between text-sm">
                <span className="text-gray-400">Ends In:</span>
                <span className="text-green-400 font-bold">{selectedLaunch.days_left} days</span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-gray-400">Stage:</span>
                <Badge>{selectedLaunch.stage}</Badge>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-gray-400">Filled:</span>
                <span className="font-bold text-x3-orange">{selectedLaunch.progress}%</span>
              </div>
            </div>
          </div>

          {/* Action Buttons */}
          <div className="space-y-2">
            <Button className="w-full px-4 py-3 bg-green-600 hover:bg-green-700 text-white font-bold rounded-lg">
              💳 Buy Tokens Now
            </Button>
            <Button
              variant="secondary"
              className="w-full px-4 py-3 text-white rounded-lg"
            >
              ⭐ Add to Watchlist
            </Button>
          </div>
        </div>
      </div>

      {/* Fundraising Models */}
      <div>
        <h2 className="text-2xl font-bold text-white mb-4">🏆 Fundraising Models</h2>
        <p className="text-gray-400 mb-6">Choose the perfect model for your Web3 project launch</p>

        <div className="grid grid-cols-4 gap-4">
          {FUNDRAISING_MODELS.map((model, idx) => (
            <div
              key={idx}
              onClick={() => setSelectedModel(idx)}
              className={`p-4 rounded-lg border cursor-pointer transition-all ${
                selectedModel === idx
                  ? 'bg-gradient-to-br from-x3-orange to-orange-700 border-x3-orange'
                  : 'bg-x3-dark border-x3-dark-gray hover:border-x3-orange'
              }`}
            >
              <div className="text-3xl mb-3">{model.icon}</div>
              <h3 className={`font-bold mb-2 ${selectedModel === idx ? 'text-white' : 'text-gray-200'}`}>
                {model.title}
              </h3>
              <p className={`text-xs mb-3 ${selectedModel === idx ? 'text-white' : 'text-gray-400'}`}>
                {model.description}
              </p>
              <ul className={`text-xs space-y-1 ${selectedModel === idx ? 'text-white' : 'text-gray-400'}`}>
                {model.features.map((feature, i) => (
                  <li key={i}>✓ {feature}</li>
                ))}
              </ul>
            </div>
          ))}
        </div>
      </div>

      {/* Launch History */}
      <div>
        <h2 className="text-2xl font-bold text-white mb-4">📈 Launchpad Performance</h2>
        <div className="bg-gradient-to-b from-x3-dark to-x3-dark-gray p-6 rounded-lg border border-x3-dark-gray">
          <ResponsiveContainer width="100%" height={300}>
            <BarChart data={RAISED_CHART}>
              <CartesianGrid strokeDasharray="3 3" stroke="#2a2a2e" />
              <XAxis dataKey="month" stroke="#8a8a8e" />
              <YAxis stroke="#8a8a8e" />
              <Tooltip
                contentStyle={{ backgroundColor: '#1a1a1d', border: '1px solid #00d4aa' }}
                labelStyle={{ color: '#00d4aa' }}
              />
              <Bar dataKey="raised" fill="#00d4aa" radius={[8, 8, 0, 0]} />
            </BarChart>
          </ResponsiveContainer>
        </div>
      </div>

      {/* Submit Project Modal */}
      {showSubmitForm && (
        <div className="fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center z-50">
          <div className="bg-x3-dark border border-x3-dark-gray rounded-lg p-6 max-w-2xl w-full mx-4 max-h-[90vh] overflow-y-auto">
            <div className="flex justify-between items-center mb-6">
              <h2 className="text-2xl font-bold text-white">🎯 Submit Your Project</h2>
              <button
                onClick={() => setShowSubmitForm(false)}
                className="text-gray-400 hover:text-white text-2xl"
              >
                ✕
              </button>
            </div>

            <div className="space-y-4">
              <div>
                <label className="block text-white font-bold mb-2">Project Name</label>
                <input
                  type="text"
                  placeholder="Your Token/Project Name"
                  className="w-full px-4 py-2 bg-x3-dark-gray border border-x3-dark-gray rounded-lg text-white placeholder-gray-500"
                />
              </div>

              <div>
                <label className="block text-white font-bold mb-2">Token Ticker</label>
                <input
                  type="text"
                  placeholder="e.g., TOKEN"
                  maxLength={10}
                  className="w-full px-4 py-2 bg-x3-dark-gray border border-x3-dark-gray rounded-lg text-white placeholder-gray-500"
                />
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-white font-bold mb-2">Total Supply</label>
                  <input
                    type="number"
                    placeholder="1,000,000"
                    className="w-full px-4 py-2 bg-x3-dark-gray border border-x3-dark-gray rounded-lg text-white placeholder-gray-500"
                  />
                </div>
                <div>
                  <label className="block text-white font-bold mb-2">Fundraising Goal (USD)</label>
                  <input
                    type="number"
                    placeholder="1,000,000"
                    className="w-full px-4 py-2 bg-x3-dark-gray border border-x3-dark-gray rounded-lg text-white placeholder-gray-500"
                  />
                </div>
              </div>

              <div>
                <label className="block text-white font-bold mb-2">Select Fundraising Model</label>
                <select className="w-full px-4 py-2 bg-x3-dark-gray border border-x3-dark-gray rounded-lg text-white">
                  <option>Traditional IDO</option>
                  <option>Fair Launch</option>
                  <option>Revenue Sharing</option>
                  <option>Hybrid Model</option>
                </select>
              </div>

              <div>
                <label className="block text-white font-bold mb-2">Project Description</label>
                <textarea
                  placeholder="Tell us about your project..."
                  rows={4}
                  className="w-full px-4 py-2 bg-x3-dark-gray border border-x3-dark-gray rounded-lg text-white placeholder-gray-500"
                ></textarea>
              </div>

              <div className="space-y-2">
                <Button className="w-full px-4 py-3 bg-x3-orange hover:bg-orange-600 text-white font-bold rounded-lg">
                  ✅ Submit Application
                </Button>
                <Button
                  onClick={() => setShowSubmitForm(false)}
                  variant="secondary"
                  className="w-full px-4 py-3 text-white rounded-lg"
                >
                  Cancel
                </Button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
