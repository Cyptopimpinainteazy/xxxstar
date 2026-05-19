// crates/gpu-swarm/src/monitoring/metrics.rs
// Prometheus metrics for GPU Swarm monitoring

use prometheus::{
    Counter, CounterVec, Encoder, Gauge, GaugeVec, Histogram, HistogramOpts, HistogramVec, Opts,
    Registry,
};

pub struct MetricsCollector {
    registry: Registry,

    // GPU Metrics
    pub gpu_utilization: GaugeVec,
    pub gpu_temperature: GaugeVec,
    pub gpu_memory_used: GaugeVec,
    pub gpu_memory_total: GaugeVec,
    pub gpu_power_consumption: GaugeVec,
    pub gpu_thermal_throttle_events: CounterVec,
    pub gpu_error_count: CounterVec,

    // Task Metrics
    pub task_queue_depth: Gauge,
    pub task_submitted_total: Counter,
    pub task_completed_total: Counter,
    // Backwards-compatible aliases expected by tests
    pub tasks_submitted: Counter,
    pub tasks_completed: Counter,
    pub task_failed_total: Counter,
    pub task_execution_time: Histogram,
    pub task_execution_time_by_type: HistogramVec,

    // Network Metrics
    pub peer_count: Gauge,
    pub peer_connections: CounterVec,
    pub peer_disconnections: CounterVec,
    pub network_latency: Histogram,
    pub network_bandwidth: GaugeVec,
    pub network_packet_loss: GaugeVec,

    // Coordinator Metrics
    pub coordinator_memory_usage: Gauge,
    pub coordinator_cpu_usage: Gauge,
    pub coordinator_request_latency: Histogram,
    pub coordinator_requests_total: CounterVec,

    // Economic Metrics
    pub reward_distributed_total: Counter,
    pub slashing_events_total: Counter,
    pub reward_fund_balance: Gauge,
    pub node_stake: GaugeVec,

    // Health Metrics
    pub uptime_seconds: Counter,
    pub health_check_failures: CounterVec,
    pub database_query_latency: HistogramVec,
}

impl MetricsCollector {
    /// Construct a collector from a provided `Registry`.
    pub fn new_with_registry(registry: Registry) -> Result<Self, Box<dyn std::error::Error>> {
        // GPU Metrics
        let gpu_utilization = GaugeVec::new(
            Opts::new("gpu_utilization_percent", "GPU utilization percentage"),
            &["device", "node", "backend"],
        )?;
        registry.register(Box::new(gpu_utilization.clone()))?;

        let gpu_temperature = GaugeVec::new(
            Opts::new("gpu_temperature_celsius", "GPU temperature in Celsius"),
            &["device", "node"],
        )?;
        registry.register(Box::new(gpu_temperature.clone()))?;

        let gpu_memory_used = GaugeVec::new(
            Opts::new("gpu_memory_used_bytes", "GPU memory used in bytes"),
            &["device", "node"],
        )?;
        registry.register(Box::new(gpu_memory_used.clone()))?;

        let gpu_memory_total = GaugeVec::new(
            Opts::new("gpu_memory_total_bytes", "GPU total memory in bytes"),
            &["device", "node"],
        )?;
        registry.register(Box::new(gpu_memory_total.clone()))?;

        let gpu_power_consumption = GaugeVec::new(
            Opts::new(
                "gpu_power_consumption_watts",
                "GPU power consumption in watts",
            ),
            &["device", "node"],
        )?;
        registry.register(Box::new(gpu_power_consumption.clone()))?;

        let gpu_thermal_throttle_events = CounterVec::new(
            Opts::new(
                "gpu_thermal_throttle_events_total",
                "GPU thermal throttle events",
            ),
            &["device", "node"],
        )?;
        registry.register(Box::new(gpu_thermal_throttle_events.clone()))?;

        let gpu_error_count = CounterVec::new(
            Opts::new("gpu_error_count_total", "Total GPU errors"),
            &["device", "node", "error_type"],
        )?;
        registry.register(Box::new(gpu_error_count.clone()))?;

        // Task Metrics
        let task_queue_depth = Gauge::new("gpu_task_queue_depth", "Current task queue depth")?;
        registry.register(Box::new(task_queue_depth.clone()))?;

        let task_submitted_total =
            Counter::new("gpu_task_submitted_total", "Total tasks submitted")?;
        registry.register(Box::new(task_submitted_total.clone()))?;

        let task_completed_total = Counter::new(
            "gpu_task_completed_total",
            "Total tasks completed successfully",
        )?;
        registry.register(Box::new(task_completed_total.clone()))?;

        let task_failed_total = Counter::new("gpu_task_failed_total", "Total tasks failed")?;
        registry.register(Box::new(task_failed_total.clone()))?;

        let task_execution_time = Histogram::with_opts(
            HistogramOpts::new(
                "gpu_task_execution_time_seconds",
                "Task execution time in seconds",
            )
            .buckets(vec![0.1, 0.5, 1.0, 5.0, 10.0, 30.0, 60.0, 300.0, 3600.0]),
        )?;
        registry.register(Box::new(task_execution_time.clone()))?;

        let task_execution_time_by_type = HistogramVec::new(
            HistogramOpts::new(
                "gpu_task_execution_time_by_type_seconds",
                "Task execution time by type",
            )
            .buckets(vec![0.1, 0.5, 1.0, 5.0, 10.0, 30.0, 60.0, 300.0]),
            &["task_type"],
        )?;
        registry.register(Box::new(task_execution_time_by_type.clone()))?;

        // Network Metrics
        let peer_count = Gauge::new("gpu_peer_count", "Number of connected peers")?;
        registry.register(Box::new(peer_count.clone()))?;

        let peer_connections = CounterVec::new(
            Opts::new(
                "gpu_peer_connections_total",
                "Total peer connections established",
            ),
            &["peer_type"],
        )?;
        registry.register(Box::new(peer_connections.clone()))?;

        let peer_disconnections = CounterVec::new(
            Opts::new("gpu_peer_disconnections_total", "Total peer disconnections"),
            &["peer_type", "reason"],
        )?;
        registry.register(Box::new(peer_disconnections.clone()))?;

        let network_latency = Histogram::with_opts(
            HistogramOpts::new("gpu_network_latency_ms", "Network latency in milliseconds")
                .buckets(vec![10.0, 50.0, 100.0, 200.0, 500.0, 1000.0]),
        )?;
        registry.register(Box::new(network_latency.clone()))?;

        let network_bandwidth = GaugeVec::new(
            Opts::new(
                "gpu_network_bandwidth_bytes_per_sec",
                "Network bandwidth in bytes/sec",
            ),
            &["direction", "peer"],
        )?;
        registry.register(Box::new(network_bandwidth.clone()))?;

        let network_packet_loss = GaugeVec::new(
            Opts::new(
                "gpu_network_packet_loss_percent",
                "Network packet loss percentage",
            ),
            &["peer"],
        )?;
        registry.register(Box::new(network_packet_loss.clone()))?;

        // Coordinator Metrics
        let coordinator_memory_usage = Gauge::new(
            "coordinator_memory_usage_bytes",
            "Coordinator memory usage in bytes",
        )?;
        registry.register(Box::new(coordinator_memory_usage.clone()))?;

        let coordinator_cpu_usage = Gauge::new(
            "coordinator_cpu_usage_percent",
            "Coordinator CPU usage percentage",
        )?;
        registry.register(Box::new(coordinator_cpu_usage.clone()))?;

        let coordinator_request_latency = Histogram::with_opts(
            HistogramOpts::new(
                "coordinator_request_latency_ms",
                "Coordinator request latency in milliseconds",
            )
            .buckets(vec![1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0]),
        )?;
        registry.register(Box::new(coordinator_request_latency.clone()))?;

        let coordinator_requests_total = CounterVec::new(
            Opts::new("coordinator_requests_total", "Total coordinator requests"),
            &["method", "status"],
        )?;
        registry.register(Box::new(coordinator_requests_total.clone()))?;

        // Economic Metrics
        let reward_distributed_total = Counter::new(
            "reward_distributed_total",
            "Total tokens distributed as rewards",
        )?;
        registry.register(Box::new(reward_distributed_total.clone()))?;

        let slashing_events_total = Counter::new("slashing_events_total", "Total slashing events")?;
        registry.register(Box::new(slashing_events_total.clone()))?;

        let reward_fund_balance = Gauge::new(
            "reward_fund_balance_tokens",
            "Current balance of reward fund in tokens",
        )?;
        registry.register(Box::new(reward_fund_balance.clone()))?;

        let node_stake = GaugeVec::new(
            Opts::new("node_stake_tokens", "Node stake in tokens"),
            &["node"],
        )?;
        registry.register(Box::new(node_stake.clone()))?;

        // Health Metrics
        let uptime_seconds = Counter::new("uptime_seconds", "System uptime in seconds")?;
        registry.register(Box::new(uptime_seconds.clone()))?;

        let health_check_failures = CounterVec::new(
            Opts::new("health_check_failures_total", "Total health check failures"),
            &["component"],
        )?;
        registry.register(Box::new(health_check_failures.clone()))?;

        let database_query_latency = HistogramVec::new(
            HistogramOpts::new(
                "database_query_latency_ms",
                "Database query latency in milliseconds",
            )
            .buckets(vec![1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 1000.0]),
            &["query_type"],
        )?;
        registry.register(Box::new(database_query_latency.clone()))?;

        // create aliases for clone-before-move to avoid borrow-after-move
        let tasks_submitted_alias = task_submitted_total.clone();
        let tasks_completed_alias = task_completed_total.clone();

        Ok(Self {
            registry,
            gpu_utilization,
            gpu_temperature,
            gpu_memory_used,
            gpu_memory_total,
            gpu_power_consumption,
            gpu_thermal_throttle_events,
            gpu_error_count,
            task_queue_depth,
            task_submitted_total,
            // aliases for older tests
            tasks_submitted: tasks_submitted_alias,
            task_completed_total,
            tasks_completed: tasks_completed_alias,
            task_failed_total,
            task_execution_time,
            task_execution_time_by_type,
            peer_count,
            peer_connections,
            peer_disconnections,
            network_latency,
            network_bandwidth,
            network_packet_loss,
            coordinator_memory_usage,
            coordinator_cpu_usage,
            coordinator_request_latency,
            coordinator_requests_total,
            reward_distributed_total,
            slashing_events_total,
            reward_fund_balance,
            node_stake,
            uptime_seconds,
            health_check_failures,
            database_query_latency,
        })
    }
}

impl MetricsCollector {
    /// Backwards-compatible constructor used by tests (isolated registry)
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self::new_with_registry(Registry::new())?)
    }

    /// Gather text-format metrics from this collector's registry
    pub fn gather_metrics(&self) -> String {
        let encoder = prometheus::TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        let _ = encoder.encode(&metric_families, &mut buffer);
        String::from_utf8_lossy(&buffer).to_string()
    }
}
