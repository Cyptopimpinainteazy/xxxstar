import { startServer } from './index';
import { HealthMonitor } from '../connector/health-monitor';

// quick dev server that instantiates a monitor for local inspection
const monitor = new HealthMonitor({ concurrency: 20, timeoutMs: 5000, intervalMs: 30_000 });
startServer({ monitor, port: 9464 });
console.log('dev server started');
