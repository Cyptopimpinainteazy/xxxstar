import React, { useState } from 'react';
import { Music, Play, Pause, SkipForward, Volume2, Heart, Share2, Download, User, TrendingUp } from 'lucide-react';

interface Track {
  id: string;
  title: string;
  artist: string;
  duration: number;
  streams: number;
  earnings: number;
  image: string;
  genre: string;
  releaseDate: string;
}

interface CreatorProfile {
  id: string;
  name: string;
  avatar: string;
  followers: number;
  totalEarnings: number;
  monthlyStreams: number;
  topTrack: string;
}

export const MediaStreamingPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'discover' | 'creator' | 'library'>('discover');
  const [isPlaying, setIsPlaying] = useState(false);

  const tracks: Track[] = [
    {
      id: 'track-1',
      title: 'Digital Dreams',
      artist: 'Luna Synthwave',
      duration: 234,
      streams: 45200,
      earnings: 2.26,
      image: '🎵',
      genre: 'Electronic',
      releaseDate: '2025-12-01',
    },
    {
      id: 'track-2',
      title: 'Blockchain Vibes',
      artist: 'CryptoWave',
      duration: 198,
      streams: 32100,
      earnings: 1.60,
      image: '🎸',
      genre: 'Hip-Hop',
      releaseDate: '2025-11-15',
    },
    {
      id: 'track-3',
      title: 'Decentralized',
      artist: 'Web3 Collective',
      duration: 267,
      streams: 18900,
      earnings: 0.95,
      image: '🎹',
      genre: 'Ambient',
      releaseDate: '2025-10-20',
    },
  ];

  const creators: CreatorProfile[] = [
    {
      id: 'creator-1',
      name: 'Luna Synthwave',
      avatar: '👩‍🎤',
      followers: 12450,
      totalEarnings: 5240,
      monthlyStreams: 250000,
      topTrack: 'Digital Dreams',
    },
    {
      id: 'creator-2',
      name: 'CryptoWave',
      avatar: '👨‍🎤',
      followers: 8340,
      totalEarnings: 3120,
      monthlyStreams: 180000,
      topTrack: 'Blockchain Vibes',
    },
  ];

  return (
    <div className="h-full flex flex-col bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f]">
      {/* Header */}
      <div className="border-b border-[#2a2a35] p-4">
        <div className="flex items-center gap-3 mb-4">
          <div className="p-2 bg-gradient-to-br from-pink-500 to-rose-500 rounded-lg">
            <Music className="w-5 h-5 text-white" />
          </div>
          <div>
            <h1 className="text-lg font-semibold text-white">Media Streaming</h1>
            <p className="text-xs text-gray-400">Decentralized music with creator micropayments</p>
          </div>
        </div>
      </div>

      {/* Tabs */}
      <div className="flex gap-2 px-4 pt-4 border-b border-[#2a2a35]">
        {(['discover', 'creator', 'library'] as const).map(tab => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            className={`px-4 py-2 rounded-lg font-medium text-sm transition ${
              activeTab === tab
                ? 'bg-pink-600 text-white'
                : 'text-gray-400 hover:text-gray-200'
            }`}
          >
            {tab === 'discover' && '🎵 Discover'}
            {tab === 'creator' && '👨‍🎤 Creators'}
            {tab === 'library' && '📚 My Library'}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        {activeTab === 'discover' && (
          <div className="p-4 space-y-4">
            <input
              type="text"
              placeholder="Search tracks..."
              className="w-full bg-[#0a0a0f] border border-[#2a2a35] rounded-lg px-4 py-2 text-white placeholder-gray-600 focus:border-pink-500 outline-none"
            />

            {tracks.map(track => (
              <div key={track.id} className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4 hover:border-pink-500/50 transition">
                <div className="flex items-start gap-4 mb-3">
                  <div className="text-4xl">{track.image}</div>
                  <div className="flex-1">
                    <h3 className="font-semibold text-white">{track.title}</h3>
                    <p className="text-xs text-gray-400">{track.artist}</p>
                    <div className="flex gap-2 mt-1">
                      <span className="text-xs px-2 py-0.5 bg-pink-500/20 text-pink-400 rounded">
                        {track.genre}
                      </span>
                      <span className="text-xs px-2 py-0.5 bg-purple-500/20 text-purple-400 rounded">
                        {Math.floor(track.duration / 60)}:{(track.duration % 60).toString().padStart(2, '0')}
                      </span>
                    </div>
                  </div>
                </div>

                <div className="bg-[#0a0a0f] rounded p-3 mb-3 border border-[#2a2a35]">
                  <div className="grid grid-cols-3 gap-2 mb-3 text-xs">
                    <div>
                      <p className="text-gray-500 mb-1">Streams</p>
                      <p className="font-semibold text-white">{(track.streams / 1000).toFixed(1)}K</p>
                    </div>
                    <div>
                      <p className="text-gray-500 mb-1">Earnings</p>
                      <p className="font-semibold text-cyan-400">{track.earnings.toFixed(2)} X3</p>
                    </div>
                    <div>
                      <p className="text-gray-500 mb-1">Released</p>
                      <p className="font-semibold text-white">{track.releaseDate}</p>
                    </div>
                  </div>

                  <div className="w-full h-1 bg-gray-700 rounded-full">
                    <div className="h-1 bg-pink-500 rounded-full" style={{ width: '35%' }} />
                  </div>
                </div>

                <div className="flex gap-2">
                  <button
                    onClick={() => setIsPlaying(!isPlaying)}
                    className="flex-1 px-4 py-2 bg-pink-600 hover:bg-pink-700 text-white rounded font-medium transition flex items-center justify-center gap-2"
                  >
                    {isPlaying ? <Pause size={16} /> : <Play size={16} />}
                    {isPlaying ? 'Pause' : 'Play'}
                  </button>
                  <button className="px-4 py-2 bg-[#0a0a0f] border border-[#2a2a35] rounded text-gray-300 hover:border-gray-500 transition">
                    <Heart size={16} />
                  </button>
                  <button className="px-4 py-2 bg-[#0a0a0f] border border-[#2a2a35] rounded text-gray-300 hover:border-gray-500 transition">
                    <Share2 size={16} />
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}

        {activeTab === 'creator' && (
          <div className="p-4 space-y-4">
            {creators.map(creator => (
              <div key={creator.id} className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4 hover:border-pink-500/50 transition">
                <div className="flex items-start gap-4 mb-3">
                  <div className="text-5xl">{creator.avatar}</div>
                  <div className="flex-1">
                    <h3 className="font-semibold text-white text-lg">{creator.name}</h3>
                    <div className="flex items-center gap-3 mt-2 text-xs">
                      <span className="text-gray-400">
                        <User size={12} className="inline mr-1" />
                        {(creator.followers / 1000).toFixed(1)}K followers
                      </span>
                      <span className="text-gray-400">
                        <TrendingUp size={12} className="inline mr-1" />
                        {(creator.monthlyStreams / 1000).toFixed(0)}K streams/mo
                      </span>
                    </div>
                  </div>
                </div>

                <div className="bg-[#0a0a0f] rounded p-3 mb-3 border border-[#2a2a35]">
                  <div className="grid grid-cols-2 gap-3">
                    <div>
                      <p className="text-xs text-gray-500 mb-1">Total Earned</p>
                      <p className="text-lg font-semibold text-cyan-400">{creator.totalEarnings.toFixed(0)} X3</p>
                    </div>
                    <div>
                      <p className="text-xs text-gray-500 mb-1">Top Track</p>
                      <p className="text-sm font-semibold text-white">{creator.topTrack}</p>
                    </div>
                  </div>
                </div>

                <button className="w-full px-4 py-2 bg-pink-600 hover:bg-pink-700 text-white rounded font-medium transition text-sm">
                  Subscribe (0.99 X3/month)
                </button>
              </div>
            ))}

            <div className="bg-blue-500/10 border border-blue-500/30 rounded-lg p-4">
              <h3 className="font-semibold text-white mb-2">Become a Creator</h3>
              <p className="text-xs text-gray-400 mb-3">Upload your music and earn 0.001 X3 per 30s of listening</p>
              <button className="w-full px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded font-medium transition text-sm">
                Upload Track
              </button>
            </div>
          </div>
        )}

        {activeTab === 'library' && (
          <div className="p-4">
            <div className="text-center py-12 text-gray-400">
              <Music className="w-12 h-12 mx-auto mb-3 text-gray-500" />
              <p className="text-sm">Your library is empty</p>
              <p className="text-xs mt-1">Add tracks to your library from discover</p>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default MediaStreamingPanel;
