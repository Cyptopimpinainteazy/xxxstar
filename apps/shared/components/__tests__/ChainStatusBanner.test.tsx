import React from 'react';
import { render, screen } from '@testing-library/react';
import '@testing-library/jest-dom';
import { ChainStatusBanner } from '../ChainStatusBanner';

describe('ChainStatusBanner', () => {
  it('renders status and connected state', () => {
    render(<ChainStatusBanner status="Running" isConnected={true} />);
    expect(screen.getByText(/Running/)).toBeInTheDocument();
    expect(screen.getByText(/Connected/)).toBeInTheDocument();
  });
});
