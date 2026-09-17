"use client";
import { useState } from "react";

const ITEMS = [
  ["Architecture", "#architecture"],
  ["Status", "#status"],
  ["Security", "#security"],
  ["Funding & Grants", "#funding"],
  ["Ecosystem", "#ecosystem"],
  ["Contact", "#contact"],
];

export default function MobileNav() {
  const [open, setOpen] = useState(false);
  return (
    <div className="mobile-nav">
      <button
        className="mobile-nav-toggle"
        aria-label="Toggle navigation"
        aria-expanded={open}
        onClick={() => setOpen((v) => !v)}
      >
        <span />
        <span />
        <span />
      </button>
      {open && (
        <div className="mobile-nav-panel">
          {ITEMS.map(([label, href]) => (
            <a key={href} href={href} onClick={() => setOpen(false)}>
              {label}
            </a>
          ))}
        </div>
      )}
    </div>
  );
}
