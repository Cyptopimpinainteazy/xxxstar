import React, { useState, useRef, useEffect } from 'react';
import { Play, Pause, Volume2, Slider, BarChart3 } from 'lucide-react';

interface AudioVisualizerState {
  isPlaying: boolean;
  volume: number;
  currentTime: number;
  duration: number;
  frequencies: number[];
}

export const AudioVisualizerPanel: React.FC = () => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const audioContextRef = useRef<AudioContext | null>(null);
  const analyserRef = useRef<AnalyserNode | null>(null);
  const oscillatorRef = useRef<OscillatorNode | null>(null);
  
  const [state, setState] = useState<AudioVisualizerState>({
    isPlaying: false,
    volume: 0.5,
    currentTime: 0,
    duration: 0,
    frequencies: Array(64).fill(0),
  });

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const animate = () => {
      ctx.fillStyle = '#0a0a0f';
      ctx.fillRect(0, 0, canvas.width, canvas.height);

      if (analyserRef.current) {
        const dataArray = new Uint8Array(analyserRef.current.frequencyBinCount);
        analyserRef.current.getByteFrequencyData(dataArray);

        const barWidth = canvas.width / dataArray.length;
        dataArray.forEach((value, i) => {
          const height = (value / 255) * canvas.height;
          const hue = (i / dataArray.length) * 360;
          ctx.fillStyle = `hsl(${hue}, 100%, 50%)`;
          ctx.fillRect(i * barWidth, canvas.height - height, barWidth - 2, height);
        });
      }

      requestAnimationFrame(animate);
    };

    animate();
  }, []);

  const togglePlayback = () => {
    if (!state.isPlaying) {
      if (!audioContextRef.current) {
        audioContextRef.current = new (window.AudioContext || (window as any).webkitAudioContext)();
        analyserRef.current = audioContextRef.current.createAnalyser();
        analyserRef.current.connect(audioContextRef.current.destination);
      }

      const oscillator = audioContextRef.current.createOscillator();
      const gain = audioContextRef.current.createGain();
      
      oscillator.connect(gain);
      gain.connect(analyserRef.current!);
      gain.gain.value = state.volume;

      oscillator.frequency.value = 440;
      oscillator.start();

      oscillatorRef.current = oscillator;
      setState((prev) => ({ ...prev, isPlaying: true }));
    } else {
      if (oscillatorRef.current) {
        oscillatorRef.current.stop();
        oscillatorRef.current = null;
      }
      setState((prev) => ({ ...prev, isPlaying: false }));
    }
  };

  const handleVolumeChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const volume = parseFloat(e.target.value);
    setState((prev) => ({ ...prev, volume }));
    if (analyserRef.current?.context) {
      // Volume adjustment would go here
    }
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f] p-6">
      <div className="max-w-6xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-4xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-cyan-400 to-blue-500 mb-2">
              Audio Visualizer
            </h1>
            <p className="text-gray-400">Real-time frequency spectrum analyzer</p>
          </div>
          <BarChart3 className="w-12 h-12 text-cyan-400" />
        </div>

        {/* Visualizer Canvas */}
        <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg overflow-hidden mb-6">
          <canvas
            ref={canvasRef}
            width={800}
            height={300}
            className="w-full"
          />
        </div>

        {/* Controls */}
        <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
          {/* Play Controls */}
          <div className="flex items-center gap-4 mb-6">
            <button
              onClick={togglePlayback}
              className={`p-3 rounded-full transition ${
                state.isPlaying
                  ? 'bg-red-600 hover:bg-red-700'
                  : 'bg-cyan-600 hover:bg-cyan-700'
              } text-white`}
            >
              {state.isPlaying ? (
                <Pause className="w-6 h-6" />
              ) : (
                <Play className="w-6 h-6" />
              )}
            </button>
            <div className="flex-1">
              <div className="flex items-center justify-between text-xs text-gray-400 mb-1">
                <span>Frequency: 440 Hz (A4)</span>
                <span>Duration: 0:00 / 0:00</span>
              </div>
              <div className="bg-[#0a0a0f] rounded-full h-1 overflow-hidden">
                <div
                  className="h-full bg-gradient-to-r from-cyan-500 to-blue-500 transition"
                  style={{ width: `${(state.currentTime / (state.duration || 1)) * 100}%` }}
                />
              </div>
            </div>
          </div>

          {/* Volume Control */}
          <div className="flex items-center gap-4 mb-6 pb-6 border-b border-[#2a2a35]">
            <Volume2 className="w-5 h-5 text-gray-400" />
            <input
              type="range"
              min="0"
              max="1"
              step="0.01"
              value={state.volume}
              onChange={handleVolumeChange}
              className="flex-1 h-2 bg-[#0a0a0f] rounded-full appearance-none cursor-pointer accent-cyan-500"
            />
            <span className="text-gray-400 text-sm w-12 text-right">
              {Math.round(state.volume * 100)}%
            </span>
          </div>

          {/* Info Grid */}
          <div className="grid grid-cols-4 gap-4">
            <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-3">
              <div className="text-gray-400 text-xs mb-1">Current Frequency</div>
              <div className="text-cyan-400 font-bold">440 Hz</div>
            </div>
            <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-3">
              <div className="text-gray-400 text-xs mb-1">Peak Level</div>
              <div className="text-blue-400 font-bold">-3.2 dB</div>
            </div>
            <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-3">
              <div className="text-gray-400 text-xs mb-1">Sample Rate</div>
              <div className="text-teal-400 font-bold">44.1 kHz</div>
            </div>
            <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-3">
              <div className="text-gray-400 text-xs mb-1">Status</div>
              <div className={`font-bold ${state.isPlaying ? 'text-red-400' : 'text-gray-400'}`}>
                {state.isPlaying ? 'Playing' : 'Idle'}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default AudioVisualizerPanel;
