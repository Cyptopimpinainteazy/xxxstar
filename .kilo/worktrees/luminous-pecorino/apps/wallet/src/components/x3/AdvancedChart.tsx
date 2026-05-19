'use client';

import React, { useMemo } from 'react';
import {
  LineChart,
  Line,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  ComposedChart,
} from 'recharts';

export interface CandleData {
  time: string;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
  sma20?: number | null;
  ema12?: number | null;
  rsi?: number | null;
}

export interface AdvancedChartProps {
  data: CandleData[];
  height?: number;
  showIndicators?: boolean;
  indicator?: 'SMA' | 'EMA' | 'RSI' | 'MACD' | 'BOLLINGER';
}

// Calculate SMA (Simple Moving Average)
const calculateSMA = (data: CandleData[], period: number) => {
  if (!data || data.length === 0) return [];
  return data.map((_, i) => {
    if (i < period - 1) return null;
    const sum = data.slice(i - period + 1, i + 1).reduce((s, x) => s + x.close, 0);
    return sum / period;
  });
};

// Calculate EMA (Exponential Moving Average)
const calculateEMA = (data: CandleData[], period: number) => {
  if (!data || data.length === 0) return [];
  const k = 2 / (period + 1);
  let ema = data[0].close;
  return data.map((d) => {
    ema = d.close * k + ema * (1 - k);
    return ema;
  });
};

// Calculate RSI (Relative Strength Index)
const calculateRSI = (data: CandleData[], period: number = 14) => {
  if (!data || data.length === 0) return [];
  let gains = 0;
  let losses = 0;

  for (let i = 1; i < period + 1; i++) {
    const diff = data[i].close - data[i - 1].close;
    if (diff > 0) gains += diff;
    else losses -= diff;
  }

  let avgGain = gains / period;
  let avgLoss = losses / period;

  return data.map((d, i) => {
    if (i < period) return null;
    const diff = d.close - data[i - 1].close;
    if (diff > 0) {
      avgGain = (avgGain * (period - 1) + diff) / period;
      avgLoss = (avgLoss * (period - 1)) / period;
    } else {
      avgGain = (avgGain * (period - 1)) / period;
      avgLoss = (avgLoss * (period - 1) - diff) / period;
    }
    const rs = avgGain / avgLoss;
    return 100 - 100 / (1 + rs);
  });
};

export const CandlestickChart: React.FC<AdvancedChartProps> = ({
  data,
  height = 400,
  showIndicators = true,
  indicator = 'SMA',
}) => {
  // Enhance data with indicators
  const enhancedData = useMemo(() => {
    if (!showIndicators) return data;

    const sma20 = calculateSMA(data, 20);
    const ema12 = calculateEMA(data, 12);
    const rsi = calculateRSI(data, 14);

    return data.map((d, i) => ({
      ...d,
      sma20: sma20[i],
      ema12: ema12[i],
      rsi: rsi[i],
    }));
  }, [data, showIndicators]);

  return (
    <div className="w-full">
      {/* Main Price Chart */}
      <div className="bg-gradient-to-b from-x3-dark to-x3-dark-gray p-4 rounded-lg border border-x3-dark-gray backdrop-blur-sm">
        <ResponsiveContainer width="100%" height={height}>
          <ComposedChart data={enhancedData.length > 0 ? enhancedData : data}>
            <defs>
              <linearGradient id="colorFill" x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%" stopColor="#00d4aa" stopOpacity={0.3} />
                <stop offset="95%" stopColor="#00d4aa" stopOpacity={0} />
              </linearGradient>
            </defs>
            <CartesianGrid strokeDasharray="3 3" stroke="#2a2a2e" />
            <XAxis dataKey="time" stroke="#8a8a8e" />
            <YAxis stroke="#8a8a8e" />
            <Tooltip
              contentStyle={{
                backgroundColor: '#1a1a1d',
                border: '1px solid #00d4aa',
                borderRadius: '8px',
              }}
              labelStyle={{ color: '#00d4aa' }}
            />
            <Area
              type="monotone"
              dataKey="close"
              stroke="#00d4aa"
              fill="url(#colorFill)"
              strokeWidth={2}
            />
            {indicator === 'SMA' && enhancedData.length > 0 && enhancedData[0]?.sma20 && (
              <Line
                type="monotone"
                dataKey="sma20"
                stroke="#4488ff"
                dot={false}
                strokeWidth={1.5}
                isAnimationActive={false}
              />
            )}
            {indicator === 'EMA' && enhancedData.length > 0 && enhancedData[0]?.ema12 && (
              <Line
                type="monotone"
                dataKey="ema12"
                stroke="#ffaa00"
                dot={false}
                strokeWidth={1.5}
                isAnimationActive={false}
              />
            )}
          </ComposedChart>
        </ResponsiveContainer>
      </div>

      {/* RSI Indicator */}
      {showIndicators && indicator === 'RSI' && enhancedData.length > 0 && (
        <div className="mt-4 bg-gradient-to-b from-x3-dark to-x3-dark-gray p-4 rounded-lg border border-x3-dark-gray">
          <h3 className="text-sm font-bold text-gray-400 mb-2">RSI (14)</h3>
          <ResponsiveContainer width="100%" height={150}>
            <LineChart data={enhancedData}>
              <CartesianGrid strokeDasharray="3 3" stroke="#2a2a2e" />
              <XAxis dataKey="time" stroke="#8a8a8e" />
              <YAxis stroke="#8a8a8e" domain={[0, 100]} />
              <Line
                type="monotone"
                dataKey="rsi"
                stroke="#ff6b6b"
                dot={false}
                strokeWidth={1.5}
              />
              <Tooltip
                contentStyle={{
                  backgroundColor: '#1a1a1d',
                  border: '1px solid #ff6b6b',
                }}
              />
            </LineChart>
          </ResponsiveContainer>
        </div>
      )}
    </div>
  );
};
