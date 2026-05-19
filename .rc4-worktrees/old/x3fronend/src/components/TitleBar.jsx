import React from 'react';
import Button from './Button';

export default function TitleBar() {
  return (
    <header className="sticky top-0 z-20 border-b border-white/10 bg-[#020409]/95 backdrop-blur-lg">
      <div className="mx-auto flex max-w-7xl flex-wrap items-center justify-between gap-4 px-5 py-4 lg:px-8">
        <div className="flex items-center gap-4">
          <div className="rounded-2xl border border-white/10 bg-white/5 px-4 py-2 text-sm font-semibold tracking-[0.16em] text-cream shadow-sm shadow-cyan/10">
            X3STAR
          </div>
          <div className="hidden items-center gap-6 text-sm text-slate-300 lg:flex">
            <a className="hover:text-white" href="#featured">
              Portal
            </a>
            <a className="hover:text-white" href="#sitemap">
              Sitemap
            </a>
            <a className="hover:text-white" href="#stats">
              Live Data
            </a>
          </div>
        </div>

        <div className="flex items-center gap-3">
          <Button outline className="min-w-[150px]" onClick={() => window.location.assign('/x3star-landing.html')}>
            Launch Home
          </Button>
          <Button primary className="min-w-[150px]" onClick={() => window.location.assign('/x3star-dashboard.html')}>
            Open Dashboard
          </Button>
        </div>
      </div>
    </header>
  );
}
