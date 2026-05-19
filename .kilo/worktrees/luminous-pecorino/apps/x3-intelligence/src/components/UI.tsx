// UI Button Components

import React from "react";

interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "primary" | "secondary" | "danger" | "success";
  size?: "sm" | "md" | "lg";
  loading?: boolean;
}

export function Button({
  variant = "primary",
  size = "md",
  loading = false,
  disabled,
  children,
  className,
  ...props
}: ButtonProps) {
  const baseClasses = "btn";
  const variantClasses =
    {
      primary: "btn-primary",
      secondary: "btn-secondary",
      danger: "btn-danger",
      success: "btn-success",
    }[variant] || "";
  const sizeClasses =
    {
      sm: "btn-sm",
      md: "btn-md",
      lg: "btn-lg",
    }[size] || "";

  return (
    <button
      className={`${baseClasses} ${variantClasses} ${sizeClasses} ${className || ""}`}
      disabled={disabled || loading}
      {...props}
    >
      {loading ? "Loading..." : children}
    </button>
  );
}

interface CardProps {
  title?: string;
  children: React.ReactNode;
  className?: string;
}

export function Card({ title, children, className }: CardProps) {
  return (
    <div className={`card ${className || ""}`}>
      {title && (
        <div className="card-header">
          <h2>{title}</h2>
        </div>
      )}
      {children}
    </div>
  );
}

interface StatCardProps {
  label: string;
  value: string | number;
  color?: "green" | "red" | "amber" | "blue" | "default";
}

export function StatCard({ label, value, color = "default" }: StatCardProps) {
  const colorClass = color !== "default" ? color : "";
  return (
    <div className="stat-card">
      <div className="stat-label">{label}</div>
      <div className={`stat-value ${colorClass}`}>{value}</div>
    </div>
  );
}

interface ModalProps {
  open: boolean;
  title?: string;
  onClose: () => void;
  children: React.ReactNode;
}

export function Modal({ open, title, onClose, children }: ModalProps) {
  if (!open) return null;

  return (
    <>
      <div className="modal-overlay" onClick={onClose} />
      <div className="modal">
        {title && (
          <div className="modal-header">
            <h2>{title}</h2>
            <button className="modal-close" onClick={onClose}>
              ×
            </button>
          </div>
        )}
        <div className="modal-body">{children}</div>
      </div>
    </>
  );
}

interface InputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  error?: string;
}

export function Input({ label, error, ...props }: InputProps) {
  return (
    <div className="input-group">
      {label && <label className="input-label">{label}</label>}
      <input className="input" {...props} />
      {error && <span className="input-error">{error}</span>}
    </div>
  );
}

interface SelectProps extends React.SelectHTMLAttributes<HTMLSelectElement> {
  label?: string;
  options: Array<{ value: string; label: string }>;
}

export function Select({ label, options, ...props }: SelectProps) {
  return (
    <div className="input-group">
      {label && <label className="input-label">{label}</label>}
      <select className="input" {...props}>
        {options.map((opt) => (
          <option key={opt.value} value={opt.value}>
            {opt.label}
          </option>
        ))}
      </select>
    </div>
  );
}

interface BadgeProps {
  variant?: "green" | "red" | "amber" | "blue" | "muted";
  children: React.ReactNode;
}

export function Badge({ variant = "muted", children }: BadgeProps) {
  const variantClass = `badge-${variant}`;
  return <span className={`badge ${variantClass}`}>{children}</span>;
}

interface ProgressBarProps {
  value: number;
  max?: number;
  color?: "green" | "red" | "amber" | "blue";
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
        className={`progress-fill progress-${color}`}
        style={{ width: `${Math.min(percentage, 100)}%` }}
      />
    </div>
  );
}
