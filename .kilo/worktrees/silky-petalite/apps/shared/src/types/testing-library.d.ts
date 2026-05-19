// Minimal testing-library and jest-dom shims for type-checking tests in the monorepo
declare module '@testing-library/react' {
  import * as React from 'react';
  export function render(ui: React.ReactElement): any;
  export const screen: any;
}

// Provide minimal jest-dom matcher augmentation used by tests
declare namespace jest {
  interface Matchers<R> {
    toBeInTheDocument(): R;
  }
}
