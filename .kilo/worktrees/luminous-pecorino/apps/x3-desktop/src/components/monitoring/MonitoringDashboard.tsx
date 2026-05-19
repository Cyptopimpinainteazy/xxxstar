import React from 'react';
import { SystemMetricsPanel } from '@/components/systemMetrics/SystemMetricsPanel';
import { IpfsStoragePanel } from '@/components/ipfsStorage/IpfsStoragePanel';
import { ErrorBoundary } from '@/components/ErrorBoundary';
import { AppError } from '@/utils/errorHandler';

const MonitoringDashboard: React.FC = () => {
  const handlePanelError = (error: AppError) => {
    console.error('Panel error:', error);
    // Could send to analytics/monitoring service here
  };

  return (
    <div className="w-full h-full p-4 space-y-4 overflow-auto bg-gray-950/80 backdrop-blur">
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        <ErrorBoundary
          componentName="SystemMetricsPanel"
          onError={handlePanelError}
        >
          <SystemMetricsPanel />
        </ErrorBoundary>

        <ErrorBoundary
          componentName="IpfsStoragePanel"
          onError={handlePanelError}
        >
          <IpfsStoragePanel />
        </ErrorBoundary>
      </div>
    </div>
  );
};

export default MonitoringDashboard;
