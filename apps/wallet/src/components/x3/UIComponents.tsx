'use client';

import React, { ReactNode } from 'react';

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'danger' | 'success';
  size?: 'sm' | 'md' | 'lg';
  loading?: boolean;
  icon?: ReactNode;
}

export function Button({
  variant = 'primary',
  size = 'md',
  loading = false,
  icon,
  children,
  disabled,
  className,
  ...props
}: ButtonProps) {
  const sizeMap = { sm: 'text-sm px-2 py-1', md: 'text-base px-4 py-2', lg: 'text-lg px-6 py-3' };
  const variantMap = {
    primary: 'bg-x3-orange hover:bg-orange-600 text-white',
    secondary: 'bg-x3-dark hover:bg-x3-dark-gray text-white border border-x3-dark-gray',
    danger: 'bg-x3-red hover:bg-red-600 text-white',
    success: 'bg-x3-green hover:bg-green-600 text-white',
  };

  const finalClass = `inline-flex items-center gap-2 rounded font-medium transition-colors ${
    sizeMap[size]
  } ${variantMap[variant]} ${disabled || loading ? 'opacity-50 cursor-not-allowed' : ''} ${
    className || ''
  }`;

  return (
    <button className={finalClass} disabled={disabled || loading} {...props}>
      {icon && <span>{icon}</span>}
      {loading ? 'Loading...' : children}
    </button>
  );
}

interface BadgeProps {
  children: ReactNode;
  variant?: 'green' | 'red' | 'amber' | 'blue' | 'muted';
}

export function Badge({ children, variant = 'blue' }: BadgeProps) {
  const colorMap = {
    green: 'bg-green-900 text-green-200',
    red: 'bg-red-900 text-red-200',
    amber: 'bg-amber-900 text-amber-200',
    blue: 'bg-blue-900 text-blue-200',
    muted: 'bg-gray-700 text-gray-300',
  };

  return (
    <span className={`inline-block px-2 py-1 rounded text-xs font-medium ${colorMap[variant]}`}>
      {children}
    </span>
  );
}

interface LoadingProps {
  size?: 'sm' | 'md' | 'lg';
}

export function Loading({ size = 'md' }: LoadingProps) {
  const sizeMap = { sm: 'w-4 h-4', md: 'w-8 h-8', lg: 'w-12 h-12' };
  return (
    <div className="flex justify-center items-center p-8">
      <div className={`${sizeMap[size]} border-2 border-x3-orange border-t-transparent rounded-full animate-spin`} />
    </div>
  );
}

interface ProgressBarProps {
  value: number;
  max?: number;
  color?: 'green' | 'amber' | 'red' | 'blue';
}

export function ProgressBar({ value, max = 100, color = 'green' }: ProgressBarProps) {
  const percentage = (value / max) * 100;
  const colorMap = {
    green: 'bg-green-500',
    amber: 'bg-amber-500',
    red: 'bg-red-500',
    blue: 'bg-blue-500',
  };

  return (
    <div className="w-full h-2 bg-x3-dark-gray rounded overflow-hidden">
      <div
        className={`h-full ${colorMap[color]} transition-all`}
        style={{ width: `${Math.min(percentage, 100)}%` }}
      />
    </div>
  );
}

interface MetricProps {
  label: string;
  value: string | number;
  highlight?: boolean;
}

export function Metric({ label, value, highlight }: MetricProps) {
  return (
    <div className="mb-3">
      <div className="text-xs text-gray-400 uppercase tracking-wider">{label}</div>
      <div className={`text-lg font-semibold ${highlight ? 'text-x3-orange' : 'text-white'}`}>
        {value}
      </div>
    </div>
  );
}
