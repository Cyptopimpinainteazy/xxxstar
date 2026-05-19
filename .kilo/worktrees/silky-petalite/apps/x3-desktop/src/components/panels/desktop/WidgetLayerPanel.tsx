import React, { useState } from 'react';
import { Zap, MessageCircle, Radio, X, Settings, ChevronDown } from 'lucide-react';
import clsx from 'clsx';

interface Widget {
  id: string;
  name: string;
  enabled: boolean;
  position: 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right';
}

const WidgetLayerPanel: React.FC = () => {
  const [widgets, setWidgets] = useState<Widget[]>([
    { id: 'price', name: 'X3 Price Ticker', enabled: true, position: 'top-right' },
    { id: 'validator', name: 'Validator Status', enabled: true, position: 'top-right' },
    { id: 'messages', name: 'Message Count', enabled: true, position: 'bottom-right' },
  ]);
  const [showSettings, setShowSettings] = useState(false);
  const [expandedWidget, setExpandedWidget] = useState<string | null>(null);

  const toggleWidget = (id: string) => {
    setWidgets(widgets.map(w => w.id === id ? { ...w, enabled: !w.enabled } : w));
  };

  const handlePositionChange = (id: string, newPosition: Widget['position']) => {
    setWidgets(widgets.map(w => w.id === id ? { ...w, position: newPosition } : w));
  };

  return (
    <div className="h-full flex flex-col bg-[#0a0a0f] text-white overflow-auto">
      {/* Header */}
      <div className="flex items-center justify-between px-5 py-4 border-b border-[#1a1a1a]">
        <div className="flex items-center gap-3">
          <Zap size={18} className="text-blue-400" />
          <h1 className="text-lg font-bold">Widget Layer</h1>
          <span className="text-xs bg-blue-500/20 text-blue-400 px-2 py-0.5 rounded">Always-On-Top</span>
        </div>
        <button
          onClick={() => setShowSettings(!showSettings)}
          className={clsx(
            'p-2 rounded-lg transition-colors',
            showSettings ? 'bg-blue-500/20 text-blue-400' : 'text-gray-500 hover:text-white'
          )}
        >
          <Settings size={18} />
        </button>
      </div>

      {/* Info */}
      <div className="px-5 py-4 bg-[#111111] border-b border-[#1a1a1a]">
        <p className="text-sm text-gray-400">
          Floating widgets stay visible across all panels. Customize which widgets to show and their positions.
        </p>
      </div>

      {/* Widget Preview */}
      {!showSettings && (
        <div className="flex-1 px-5 py-6">
          <div className="relative w-full h-96 bg-[#111111] border border-[#1a1a1a] rounded-lg overflow-hidden">
            {/* Mock Window */}
            <div className="absolute inset-0 flex flex-col">
              <div className="bg-[#0a0a0f] border-b border-[#1a1a1a] px-4 py-2 text-xs text-gray-500 font-mono">
                Desktop Window (any panel active here)
              </div>
              
              {/* Mini Widgets Positioned */}
              {widgets.filter(w => w.enabled).map((widget) => {
                let posClass = '';
                if (widget.position === 'top-right') posClass = 'top-4 right-4';
                if (widget.position === 'top-left') posClass = 'top-4 left-4';
                if (widget.position === 'bottom-right') posClass = 'bottom-4 right-4';
                if (widget.position === 'bottom-left') posClass = 'bottom-4 left-4';

                return (
                  <div
                    key={widget.id}
                    className={clsx(
                      'absolute w-56 bg-[#0a0a0f] border border-[#1a1a1a] rounded-lg p-3 shadow-2xl hover:border-[#2a2a2a] transition-colors cursor-pointer',
                      posClass
                    )}
                    onClick={() => setExpandedWidget(expandedWidget === widget.id ? null : widget.id)}
                  >
                    {widget.id === 'price' && (
                      <div>
                        <div className="text-xs text-gray-500 flex items-center gap-1 mb-1">
                          <Zap size={12} /> X3 Price
                        </div>
                        <div className="text-2xl font-bold text-blue-400">$1.25</div>
                        <div className="text-xs text-green-400">↑ 5.2% (24h)</div>
                      </div>
                    )}
                    {widget.id === 'validator' && (
                      <div>
                        <div className="text-xs text-gray-500 flex items-center gap-1 mb-1">
                          <Radio size={12} className="text-green-400 animate-pulse" /> Validator
                        </div>
                        <div className="text-sm font-bold text-green-400">Online</div>
                        <div className="text-xs text-gray-500">Uptime: 99.97%</div>
                      </div>
                    )}
                    {widget.id === 'messages' && (
                      <div>
                        <div className="text-xs text-gray-500 flex items-center gap-1 mb-1">
                          <MessageCircle size={12} /> Messages
                        </div>
                        <div className="text-2xl font-bold text-blue-400">3</div>
                        <div className="text-xs text-gray-500">Unread: 1</div>
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          </div>
        </div>
      )}

      {/* Settings View */}
      {showSettings && (
        <div className="flex-1 px-5 py-6 overflow-auto">
          <div className="space-y-4">
            {widgets.map((widget) => (
              <div
                key={widget.id}
                className="bg-[#111111] border border-[#1a1a1a] rounded-lg p-4 hover:border-[#2a2a2a] transition-colors"
              >
                <div className="flex items-center justify-between mb-3">
                  <div className="font-semibold text-white">{widget.name}</div>
                  <button
                    onClick={() => toggleWidget(widget.id)}
                    className={clsx(
                      'w-12 h-6 rounded-full transition-all',
                      widget.enabled ? 'bg-blue-500' : 'bg-[#0a0a0f] border border-[#1a1a1a]'
                    )}
                  >
                    <div
                      className={clsx(
                        'w-5 h-5 rounded-full bg-white transition-transform',
                        widget.enabled ? 'translate-x-6' : 'translate-x-0.5'
                      )}
                    />
                  </button>
                </div>

                {widget.enabled && (
                  <div>
                    <label className="text-xs text-gray-500 block mb-2">Position</label>
                    <div className="grid grid-cols-2 gap-2">
                      {['top-left', 'top-right', 'bottom-left', 'bottom-right'].map((pos) => (
                        <button
                          key={pos}
                          onClick={() => handlePositionChange(widget.id, pos as Widget['position'])}
                          className={clsx(
                            'py-2 px-3 rounded-lg text-xs font-semibold transition-all capitalize',
                            widget.position === pos
                              ? 'bg-blue-500/30 border border-blue-500/60 text-blue-400'
                              : 'bg-[#0a0a0f] border border-[#1a1a1a] text-gray-400 hover:text-white'
                          )}
                        >
                          {pos.replace('-', ' ')}
                        </button>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            ))}
          </div>

          {/* Widget Info */}
          <div className="mt-6 bg-blue-500/20 border border-blue-500/40 rounded-lg p-4 text-sm text-blue-400">
            <p className="mb-2 font-semibold">💡 Tips</p>
            <ul className="text-xs space-y-1">
              <li>• Drag widgets to reposition them</li>
              <li>• Widgets stay visible across all panels</li>
              <li>• Click any widget to expand or minimize</li>
              <li>• Right-click to remove a widget</li>
            </ul>
          </div>
        </div>
      )}
    </div>
  );
};

export default WidgetLayerPanel;

