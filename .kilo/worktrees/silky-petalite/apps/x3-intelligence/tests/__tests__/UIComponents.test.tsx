import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { Button, Badge, ProgressBar, Metric } from '../../src/components/UIComponents';

describe('UI Components', () => {
  describe('Button', () => {
    it('should render button with children', () => {
      render(<Button variant="primary">Click me</Button>);
      expect(screen.getByText('Click me')).toBeInTheDocument();
    });

    it('should apply variant classes', () => {
      const { container } = render(
        <Button variant="primary">Primary</Button>
      );
      expect(container.querySelector('.btn-primary')).toBeInTheDocument();
    });

    it('should apply size classes', () => {
      const { container } = render(
        <Button size="lg">Large</Button>
      );
      expect(container.querySelector('.btn-lg')).toBeInTheDocument();
    });

    it('should be disabled when disabled prop is true', () => {
      const button = render(<Button disabled>Disabled</Button>).container.querySelector('button');
      expect(button).toBeDisabled();
    });
  });

  describe('Badge', () => {
    it('should render badge with text', () => {
      render(<Badge variant="green">Success</Badge>);
      expect(screen.getByText('Success')).toBeInTheDocument();
    });

    it('should apply variant class', () => {
      const { container } = render(<Badge variant="red">Error</Badge>);
      expect(container.querySelector('.badge-red')).toBeInTheDocument();
    });
  });

  describe('ProgressBar', () => {
    it('should render progress bar', () => {
      const { container } = render(<ProgressBar value={50} max={100} />);
      expect(container.querySelector('.progress-bar')).toBeInTheDocument();
    });

    it('should calculate correct width', () => {
      const { container } = render(<ProgressBar value={75} max={100} />);
      const fill = container.querySelector('.progress-bar-fill');
      expect(fill).toHaveStyle('width: 75%');
    });

    it('should cap width at 100%', () => {
      const { container } = render(<ProgressBar value={150} max={100} />);
      const fill = container.querySelector('.progress-bar-fill');
      expect(fill).toHaveStyle('width: 100%');
    });

    it('should apply color class', () => {
      const { container } = render(<ProgressBar value={50} max={100} color="red" />);
      // Component renders: className="progress-bar-fill red"
      const fill = container.querySelector('.progress-bar-fill.red');
      expect(fill).toBeInTheDocument();
    });
  });

  describe('Metric', () => {
    it('should render label and value', () => {
      render(<Metric label="Active Agents" value={47} />);
      expect(screen.getByText('Active Agents')).toBeInTheDocument();
      expect(screen.getByText('47')).toBeInTheDocument();
    });

    it('should apply color class', () => {
      const { container } = render(
        <Metric label="Success Rate" value="94.7%" color="green" />
      );
      // color prop adds the class to the metric-value element
      expect(container.querySelector('.metric-value.green')).toBeInTheDocument();
    });
  });
});
