// X3 Intelligence — UI Components
// Buttons, cards, and interactive elements with enhanced styling

import React, { ReactNode } from "react";

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "primary" | "secondary" | "danger" | "success";
  size?: "sm" | "md" | "lg";
  loading?: boolean;
  icon?: ReactNode;
}

export function Button({
  variant = "primary",
  size = "md",
  loading = false,
  icon,
  children,
  disabled,
  className,
  ...props
}: ButtonProps) {
  const baseClass = `btn btn-${variant} btn-${size}`;
  const finalClass = `${baseClass}${disabled || loading ? " disabled" : ""}${
    className ? ` ${className}` : ""
  }`;

  return (
    <button className={finalClass} disabled={disabled || loading} {...props}>
      {icon && <span className="btn-icon">{icon}</span>}
      {loading ? "Loading..." : children}
    </button>
  );
}

interface StatsCardRowProps {
  label: string;
  value: string | number;
  trend?: number;
  color?: "green" | "amber" | "red" | "blue";
}

export function StatsCardRow({
  label,
  value,
  trend,
  color = "green",
}: StatsCardRowProps) {
  return (
    <div className="stats-card-row">
      <div>
        <div className="stat-label">{label}</div>
        <div className={`stat-value ${color}`}>{value}</div>
      </div>
      {trend !== undefined && (
        <div className={`stat-trend ${trend > 0 ? "green" : trend < 0 ? "red" : "muted"}`}>
          {trend > 0 ? "↑" : trend < 0 ? "↓" : "→"} {Math.abs(trend)}%
        </div>
      )}
    </div>
  );
}

interface TabProps {
  label: string;
  active: boolean;
  onClick: () => void;
}

interface TabsProps {
  tabs: TabProps[];
  className?: string;
}

export function Tabs({ tabs, className = "" }: TabsProps) {
  return (
    <div className={`tabs ${className}`}>
      {tabs.map((tab) => (
        <button
          key={tab.label}
          className={`tab ${tab.active ? "active" : ""}`}
          onClick={tab.onClick}
        >
          {tab.label}
        </button>
      ))}
    </div>
  );
}

interface BadgeProps {
  children: ReactNode;
  variant?: "green" | "red" | "amber" | "blue" | "muted";
}

export function Badge({ children, variant = "blue" }: BadgeProps) {
  return <span className={`badge badge-${variant}`}>{children}</span>;
}

interface LoadingProps {
  size?: "sm" | "md" | "lg";
}

export function Loading({ size = "md" }: LoadingProps) {
  return (
    <div className={`loading loading-${size}`}>
      <div className="spinner" />
    </div>
  );
}

interface EmptyStateProps {
  icon?: ReactNode;
  title: string;
  description?: string;
  action?: {
    label: string;
    onClick: () => void;
  };
}

export function EmptyState({
  icon,
  title,
  description,
  action,
}: EmptyStateProps) {
  return (
    <div className="empty-state">
      {icon && <div className="empty-state-icon">{icon}</div>}
      <h3 className="empty-state-title">{title}</h3>
      {description && <p className="empty-state-description">{description}</p>}
      {action && (
        <Button variant="primary" onClick={action.onClick}>
          {action.label}
        </Button>
      )}
    </div>
  );
}

interface ProgressBarProps {
  value: number;
  max?: number;
  color?: "green" | "amber" | "red" | "blue";
}

export function ProgressBar({
  value,
  max = 100,
  color = "green",
}: ProgressBarProps) {
  const percentage = (value / max) * 100;
  return (
    <div className="progress-bar">
      <div
        className={`progress-bar-fill ${color}`}
        style={{ width: `${Math.min(percentage, 100)}%` }}
      />
    </div>
  );
}

interface MetricProps {
  label: string;
  value: string | number;
  unit?: string;
  highlight?: boolean;
  color?: "green" | "amber" | "red" | "blue";
}

export function Metric({ label, value, unit, highlight = false, color }: MetricProps) {
  return (
    <div className={`metric ${highlight ? "highlighted" : ""}`}>
      <div className="metric-label">{label}</div>
      <div className={`metric-value${color ? ` ${color}` : ""}`}>
        {value}
        {unit && <span className="metric-unit">{unit}</span>}
      </div>
    </div>
  );
}
