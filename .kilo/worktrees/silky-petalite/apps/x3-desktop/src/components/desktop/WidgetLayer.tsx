// Always-on-top widget layer: Price ticker, validator status, message count
// Persistent widgets that appear on top of other windows

import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export interface Widget {
  id: string;
  name: string;
  type: 'price-ticker' | 'validator-status' | 'message-count' | 'custom';
  position: { x: number; y: number };
  size: { width: number; height: number };
  always_on_top: boolean;
  visible: boolean;
  pinned: boolean;
  opacity: number;
}

interface WidgetState {
  widgets: Widget[];
  enabled: boolean;
  auto_hide_delay: number; // ms
}

interface PriceTicker {
  symbol: string;
  price: number;
  change_24h: number;
  change_pct_24h: number;
  timestamp: Date;
}

interface ValidatorStatus {
  address: string;
  status: 'active' | 'warning' | 'offline';
  uptime: number;
  blocks_produced: number;
  last_block: number;
}

export const WidgetLayer: React.FC = () => {
  const [state, setState] = useState<WidgetState>({
    widgets: [
      {
        id: 'price-ticker',
        name: 'Price Ticker',
        type: 'price-ticker',
        position: { x: 20, y: 20 },
        size: { width: 220, height: 120 },
        always_on_top: true,
        visible: true,
        pinned: true,
        opacity: 0.95,
      },
      {
        id: 'validator-status',
        name: 'Validator Status',
        type: 'validator-status',
        position: { x: 260, y: 20 },
        size: { width: 200, height: 120 },
        always_on_top: true,
        visible: true,
        pinned: true,
        opacity: 0.95,
      },
      {
        id: 'message-count',
        name: 'Messages',
        type: 'message-count',
        position: { x: 480, y: 20 },
        size: { width: 120, height: 60 },
        always_on_top: true,
        visible: true,
        pinned: false,
        opacity: 0.95,
      },
    ],
    enabled: true,
    auto_hide_delay: 5000,
  });

  const [prices, setPrices] = useState<Map<string, PriceTicker>>(new Map());
  const [validatorStatus, setValidatorStatus] = useState<ValidatorStatus | null>(null);
  const [messageCount, setMessageCount] = useState(0);
  const [draggingWidget, setDraggingWidget] = useState<string | null>(null);
  const [dragOffset, setDragOffset] = useState({ x: 0, y: 0 });

  // Subscribe to price updates
  useEffect(() => {
    let unlistener: UnlistenFn | undefined;

    const setupPriceListener = async () => {
      try {
        unlistener = await listen('price-update', (event: any) => {
          const ticker: PriceTicker = event.payload;
          setPrices(prev => new Map(prev).set(ticker.symbol, ticker));
        });
      } catch (error) {
        console.error('Failed to listen to price updates:', error);
      }
    };

    setupPriceListener();

    return () => {
      if (unlistener) unlistener();
    };
  }, []);

  // Subscribe to validator status updates
  useEffect(() => {
    let unlistener: UnlistenFn | undefined;

    const setupValidatorListener = async () => {
      try {
        unlistener = await listen('validator-status-update', (event: any) => {
          setValidatorStatus(event.payload);
        });
      } catch (error) {
        console.error('Failed to listen to validator updates:', error);
      }
    };

    setupValidatorListener();

    return () => {
      if (unlistener) unlistener();
    };
  }, []);

  // Subscribe to message count updates
  useEffect(() => {
    let unlistener: UnlistenFn | undefined;

    const setupMessageListener = async () => {
      try {
        unlistener = await listen('message-count-update', (event: any) => {
          setMessageCount(event.payload.count);
        });
      } catch (error) {
        console.error('Failed to listen to message updates:', error);
      }
    };

    setupMessageListener();

    return () => {
      if (unlistener) unlistener();
    };
  }, []);

  // Handle widget drag start
  const handleDragStart = (e: React.MouseEvent, widgetId: string) => {
    setDraggingWidget(widgetId);
    const widget = state.widgets.find(w => w.id === widgetId);
    if (widget) {
      setDragOffset({
        x: e.clientX - widget.position.x,
        y: e.clientY - widget.position.y,
      });
    }
  };

  // Handle widget dragging
  useEffect(() => {
    if (!draggingWidget) return;

    const handleMouseMove = (e: MouseEvent) => {
      setState(prev => ({
        ...prev,
        widgets: prev.widgets.map(w =>
          w.id === draggingWidget
            ? {
                ...w,
                position: {
                  x: e.clientX - dragOffset.x,
                  y: e.clientY - dragOffset.y,
                },
              }
            : w
        ),
      }));
    };

    const handleMouseUp = () => {
      setDraggingWidget(null);
    };

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);

    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };
  }, [draggingWidget, dragOffset]);

  // Toggle widget visibility
  const toggleWidget = (widgetId: string) => {
    setState(prev => ({
      ...prev,
      widgets: prev.widgets.map(w =>
        w.id === widgetId ? { ...w, visible: !w.visible } : w
      ),
    }));
  };

  // Toggle widget pin
  const togglePin = (widgetId: string) => {
    setState(prev => ({
      ...prev,
      widgets: prev.widgets.map(w =>
        w.id === widgetId ? { ...w, pinned: !w.pinned } : w
      ),
    }));
  };

  // Save widget configuration
  const saveConfiguration = async () => {
    try {
      await invoke('save_widget_configuration', { widgets: state.widgets });
    } catch (error) {
      console.error('Failed to save widget configuration:', error);
    }
  };

  const getX3Price = (): PriceTicker | undefined => prices.get('X3');
  const getETHPrice = (): PriceTicker | undefined => prices.get('ETH');
  const getBTCPrice = (): PriceTicker | undefined => prices.get('BTC');

  const x3Price = getX3Price();
  const ethPrice = getETHPrice();
  const btcPrice = getBTCPrice();

  const getStatusColor = (status: string): string => {
    switch (status) {
      case 'active':
        return '#28a745';
      case 'warning':
        return '#ffc107';
      case 'offline':
        return '#dc3545';
      default:
        return '#999';
    }
  };

  return (
    <div className="widget-layer">
      {!state.enabled ? (
        <div className="widget-disabled">
          <button
            onClick={() => setState(prev => ({ ...prev, enabled: true }))}
            className="btn-enable-widgets"
          >
            Enable Widgets
          </button>
        </div>
      ) : (
        <>
          {/* Price Ticker Widget */}
          {state.widgets
            .filter(w => w.type === 'price-ticker' && w.visible)[0] && (
            <div
              className="widget price-ticker-widget"
              style={{
                left: `${state.widgets.find(w => w.id === 'price-ticker')?.position.x}px`,
                top: `${state.widgets.find(w => w.id === 'price-ticker')?.position.y}px`,
                width: `${state.widgets.find(w => w.id === 'price-ticker')?.size.width}px`,
                height: `${state.widgets.find(w => w.id === 'price-ticker')?.size.height}px`,
                opacity: state.widgets.find(w => w.id === 'price-ticker')?.opacity,
              }}
              onMouseDown={e => handleDragStart(e, 'price-ticker')}
            >
              <div className="widget-header">
                <div className="widget-title">💱 Prices</div>
                <div className="widget-controls">
                  <button
                    className="btn-pin"
                    onClick={() => togglePin('price-ticker')}
                  >
                    {state.widgets.find(w => w.id === 'price-ticker')?.pinned ? '📌' : '📍'}
                  </button>
                  <button
                    className="btn-close"
                    onClick={() => toggleWidget('price-ticker')}
                  >
                    ✕
                  </button>
                </div>
              </div>

              <div className="price-list">
                {x3Price && (
                  <div className="price-item">
                    <span className="price-symbol">X3</span>
                    <span className="price-value">${x3Price.price.toFixed(4)}</span>
                    <span
                      className="price-change"
                      style={{
                        color: x3Price.change_24h >= 0 ? '#28a745' : '#dc3545',
                      }}
                    >
                      {x3Price.change_24h >= 0 ? '▲' : '▼'}
                      {Math.abs(x3Price.change_pct_24h).toFixed(2)}%
                    </span>
                  </div>
                )}

                {ethPrice && (
                  <div className="price-item">
                    <span className="price-symbol">ETH</span>
                    <span className="price-value">${ethPrice.price.toFixed(2)}</span>
                    <span
                      className="price-change"
                      style={{
                        color: ethPrice.change_24h >= 0 ? '#28a745' : '#dc3545',
                      }}
                    >
                      {ethPrice.change_24h >= 0 ? '▲' : '▼'}
                      {Math.abs(ethPrice.change_pct_24h).toFixed(2)}%
                    </span>
                  </div>
                )}

                {btcPrice && (
                  <div className="price-item">
                    <span className="price-symbol">BTC</span>
                    <span className="price-value">${btcPrice.price.toFixed(2)}</span>
                    <span
                      className="price-change"
                      style={{
                        color: btcPrice.change_24h >= 0 ? '#28a745' : '#dc3545',
                      }}
                    >
                      {btcPrice.change_24h >= 0 ? '▲' : '▼'}
                      {Math.abs(btcPrice.change_pct_24h).toFixed(2)}%
                    </span>
                  </div>
                )}
              </div>
            </div>
          )}

          {/* Validator Status Widget */}
          {state.widgets
            .filter(w => w.type === 'validator-status' && w.visible)[0] && (
            <div
              className="widget validator-status-widget"
              style={{
                left: `${state.widgets.find(w => w.id === 'validator-status')?.position.x}px`,
                top: `${state.widgets.find(w => w.id === 'validator-status')?.position.y}px`,
                width: `${state.widgets.find(w => w.id === 'validator-status')?.size.width}px`,
                height: `${state.widgets.find(w => w.id === 'validator-status')?.size.height}px`,
                opacity: state.widgets.find(w => w.id === 'validator-status')?.opacity,
              }}
              onMouseDown={e => handleDragStart(e, 'validator-status')}
            >
              <div className="widget-header">
                <div className="widget-title">⚡ Validator</div>
                <div className="widget-controls">
                  <button
                    className="btn-pin"
                    onClick={() => togglePin('validator-status')}
                  >
                    {state.widgets.find(w => w.id === 'validator-status')?.pinned ? '📌' : '📍'}
                  </button>
                  <button
                    className="btn-close"
                    onClick={() => toggleWidget('validator-status')}
                  >
                    ✕
                  </button>
                </div>
              </div>

              {validatorStatus ? (
                <div className="validator-info">
                  <div className="status-indicator" style={{
                    background: getStatusColor(validatorStatus.status),
                  }}></div>
                  <div className="status-text">
                    <div className="status-label">{validatorStatus.status}</div>
                    <div className="status-value">{(validatorStatus.uptime * 100).toFixed(1)}% uptime</div>
                  </div>
                  <div className="status-blocks">
                    <div className="blocks-label">Blocks: {validatorStatus.blocks_produced}</div>
                  </div>
                </div>
              ) : (
                <div className="status-loading">Loading...</div>
              )}
            </div>
          )}

          {/* Message Count Widget */}
          {state.widgets
            .filter(w => w.type === 'message-count' && w.visible)[0] && (
            <div
              className="widget message-count-widget"
              style={{
                left: `${state.widgets.find(w => w.id === 'message-count')?.position.x}px`,
                top: `${state.widgets.find(w => w.id === 'message-count')?.position.y}px`,
                width: `${state.widgets.find(w => w.id === 'message-count')?.size.width}px`,
                height: `${state.widgets.find(w => w.id === 'message-count')?.size.height}px`,
                opacity: state.widgets.find(w => w.id === 'message-count')?.opacity,
              }}
              onMouseDown={e => handleDragStart(e, 'message-count')}
            >
              <div className="widget-header">
                <div className="widget-title">💬</div>
                <div className="widget-controls">
                  <button
                    className="btn-close"
                    onClick={() => toggleWidget('message-count')}
                  >
                    ✕
                  </button>
                </div>
              </div>

              <div className="message-count-display">
                {messageCount > 0 && (
                  <div className="message-badge">{messageCount}</div>
                )}
                <div className="message-label">
                  {messageCount === 0 ? 'No messages' : `${messageCount} unread`}
                </div>
              </div>
            </div>
          )}
        </>
      )}

      <style>{`
        .widget-layer {
          position: fixed;
          top: 0;
          left: 0;
          right: 0;
          bottom: 0;
          pointer-events: none;
          z-index: 9000;
        }

        .widget-disabled {
          position: fixed;
          top: 20px;
          right: 20px;
          pointer-events: auto;
        }

        .btn-enable-widgets {
          padding: 8px 16px;
          background: #0066cc;
          color: white;
          border: none;
          border-radius: 4px;
          cursor: pointer;
        }

        .widget {
          position: fixed;
          background: white;
          border-radius: 8px;
          box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
          pointer-events: auto;
          display: flex;
          flex-direction: column;
          border: 1px solid #ddd;
          user-select: none;
        }

        .widget-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          padding: 8px 12px;
          border-bottom: 1px solid #eee;
          cursor: grab;
        }

        .widget-header:active {
          cursor: grabbing;
        }

        .widget-title {
          font-weight: 600;
          font-size: 12px;
        }

        .widget-controls {
          display: flex;
          gap: 4px;
        }

        .btn-pin,
        .btn-close {
          background: none;
          border: none;
          cursor: pointer;
          font-size: 14px;
          padding: 2px 4px;
        }

        .btn-pin:hover,
        .btn-close:hover {
          opacity: 0.7;
        }

        .price-ticker-widget {
          min-width: 200px;
        }

        .price-list {
          flex: 1;
          padding: 8px;
          display: flex;
          flex-direction: column;
          gap: 6px;
        }

        .price-item {
          display: flex;
          justify-content: space-between;
          align-items: center;
          padding: 4px 6px;
          background: #f9f9f9;
          border-radius: 4px;
          font-size: 11px;
        }

        .price-symbol {
          font-weight: 600;
          min-width: 40px;
        }

        .price-value {
          flex: 1;
          text-align: right;
          font-weight: 600;
        }

        .price-change {
          min-width: 50px;
          text-align: right;
          font-size: 10px;
        }

        .validator-status-widget {
          min-width: 180px;
        }

        .validator-info {
          flex: 1;
          padding: 12px;
          display: flex;
          flex-direction: column;
          gap: 8px;
        }

        .status-indicator {
          width: 12px;
          height: 12px;
          border-radius: 50%;
          animation: pulse 2s infinite;
        }

        @keyframes pulse {
          0%, 100% { opacity: 1; }
          50% { opacity: 0.7; }
        }

        .status-text {
          display: flex;
          justify-content: space-between;
          font-size: 11px;
        }

        .status-label {
          font-weight: 600;
          text-transform: uppercase;
        }

        .status-value {
          color: #666;
        }

        .status-blocks {
          font-size: 10px;
          color: #666;
        }

        .status-loading {
          padding: 12px;
          text-align: center;
          font-size: 11px;
          color: #999;
        }

        .message-count-widget {
          min-width: 100px;
        }

        .message-count-display {
          flex: 1;
          padding: 8px;
          display: flex;
          flex-direction: column;
          align-items: center;
          justify-content: center;
          gap: 6px;
        }

        .message-badge {
          width: 28px;
          height: 28px;
          background: #dc3545;
          color: white;
          border-radius: 50%;
          display: flex;
          align-items: center;
          justify-content: center;
          font-weight: 600;
          font-size: 12px;
        }

        .message-label {
          font-size: 10px;
          color: #666;
          text-align: center;
        }
      `}</style>
    </div>
  );
};

export default WidgetLayer;
