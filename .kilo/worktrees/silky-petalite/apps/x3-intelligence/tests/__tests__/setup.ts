import '@testing-library/jest-dom';
import { expect, afterEach, vi } from 'vitest';
import { cleanup } from '@testing-library/react';
import { render } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import React from 'react';

// Cleanup after each test
afterEach(() => {
  cleanup();
});

/**
 * Wraps render() in a MemoryRouter with React Router v7 future flags set,
 * silencing the two ⚠️ React Router Future Flag warnings in every test.
 */
export function renderWithRouter(ui: React.ReactElement, { route = '/' } = {}) {
  return render(
    React.createElement(
      MemoryRouter,
      {
        initialEntries: [route],
        future: { v7_startTransition: true, v7_relativeSplatPath: true },
      } as any,
      ui,
    )
  );
}

// Mock window.matchMedia
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: vi.fn().mockImplementation((query) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
});

// Mock IntersectionObserver
global.IntersectionObserver = class IntersectionObserver {
  constructor() {}
  disconnect() {}
  observe() {}
  takeRecords() {
    return [];
  }
  unobserve() {}
} as any;
