//! Swarm coordinator - central task management and orchestration

use crate::error::{SwarmError, SwarmResult};
use crate::node::{NodeId, NodeRegistry, NodeStatus, SwarmNode};
use crate::protocol::*;
use crate::scheduler::{SchedulerConfig, SchedulerStats, TaskScheduler};
use crate::task::{Task, TaskId, TaskStatus};
use crate::verification::{ExecutionVerifier, Verdict, VerificationConfig, VerificationSummary};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};

/// Events emitted by the coordinator
#[derive(Debug, Clone)]
pub enum CoordinatorEvent {
    /// A node joined the swarm
    NodeJoined { node_id: NodeId, region: String },

    /// A node left the swarm
    NodeLeft { node_id: NodeId, reason: String },

    /// A task was submitted
    TaskSubmitted { task_id: TaskId, priority: u8 },

    /// A task was assigned to a node
    TaskAssigned { task_id: TaskId, node_id: NodeId },

    /// A task completed
    TaskCompleted { task_id: TaskId, success: bool },

    /// Verification completed
    VerificationCompleted { task_id: TaskId, verdict: Verdict },

    /// A node was slashed
    NodeSlashed {
        node_id: NodeId,
        amount: u64,
        reason: String,
    },

    /// Metrics updated
    MetricsUpdated(CoordinatorMetrics),
}

/// Coordinator metrics
#[derive(Debug, Clone, Default)]
pub struct CoordinatorMetrics {
    /// Total nodes registered
    pub total_nodes: usize,

    /// Online nodes
    pub online_nodes: usize,

    /// Tasks in queue
    pub queued_tasks: usize,

    /// Tasks executing
    pub executing_tasks: usize,

    /// Tasks completed (last hour)
    pub completed_tasks_hour: u64,

    /// Total compute capacity
    pub total_compute_capacity: u64,

    /// Average task latency (ms)
    pub avg_task_latency_ms: u64,

    /// Total rewards distributed (last hour)
    pub rewards_distributed_hour: u64,
}

/// Coordinator configuration
#[derive(Debug, Clone)]
pub struct CoordinatorConfig {
    /// Node heartbeat timeout (seconds)
    pub heartbeat_timeout_secs: u64,

    /// Task timeout (seconds)
    pub task_timeout_secs: u64,

    /// Maximum concurrent tasks
    pub max_concurrent_tasks: usize,

    /// Minimum stake required
    pub min_stake: u64,

    /// Slashing percentage for misbehavior
    pub slash_percentage: u8,

    /// Reward pool distribution interval (seconds)
    pub reward_interval_secs: u64,

    /// Minimum fixed reward per completed task
    pub min_task_reward: u64,

    /// Extra reward per 1,000 compute units
    pub reward_per_1000_cu: u64,

    /// Slashing percentage on execution failure
    pub failure_slash_percentage: u8,

    /// Slashing percentage on task timeout
    pub timeout_slash_percentage: u8,

    /// Scheduler configuration
    pub scheduler: SchedulerConfig,

    /// Verifier configuration
    pub verifier: VerificationConfig,
}

impl Default for CoordinatorConfig {
    fn default() -> Self {
        Self {
            heartbeat_timeout_secs: 60,
            task_timeout_secs: 300,
            max_concurrent_tasks: 1000,
            min_stake: crate::MIN_NODE_STAKE,
            slash_percentage: 10,
            reward_interval_secs: 3600,
            min_task_reward: 10,
            reward_per_1000_cu: 1,
            failure_slash_percentage: 2,
            timeout_slash_percentage: 5,
            scheduler: SchedulerConfig::default(),
            verifier: VerificationConfig::default(),
        }
    }
}

/// The swarm coordinator - manages nodes, tasks, and verification
pub struct SwarmCoordinator {
    /// Configuration
    config: CoordinatorConfig,

    /// Node registry
    nodes: Arc<RwLock<NodeRegistry>>,

    /// Task scheduler
    scheduler: Arc<RwLock<TaskScheduler>>,

    /// Execution verifier
    verifier: Arc<RwLock<ExecutionVerifier>>,

    /// Event broadcaster
    event_tx: broadcast::Sender<CoordinatorEvent>,

    /// Message queue (incoming)
    message_rx: mpsc::Receiver<MessageEnvelope>,

    /// Message sender (for responses)
    message_tx: mpsc::Sender<MessageEnvelope>,

    /// Coordinator's identity
    coordinator_id: NodeId,

    /// Coordinator's Ed25519 signing key
    signing_key: ed25519_dalek::SigningKey,

    /// Metrics
    metrics: Arc<RwLock<CoordinatorMetrics>>,

    /// Running flag
    running: Arc<RwLock<bool>>,

    /// Current epoch counter
    current_epoch: Arc<AtomicU64>,

    /// Task result store: task_id -> TaskResult
    result_store: Arc<RwLock<HashMap<TaskId, TaskResult>>>,

    /// Completed task counter for hourly metrics tracking
    completed_counter: Arc<AtomicU64>,

    /// Cumulative task latency (ms) for average calculation
    cumulative_latency_ms: Arc<AtomicU64>,

    /// Rewards distributed counter
    rewards_counter: Arc<AtomicU64>,
}

impl SwarmCoordinator {
    /// Create a new coordinator
    pub fn new(
        config: CoordinatorConfig,
        coordinator_id: NodeId,
    ) -> (
        Self,
        mpsc::Sender<MessageEnvelope>,
        broadcast::Receiver<CoordinatorEvent>,
    ) {
        let (message_tx, message_rx) = mpsc::channel(1000);
        let (event_tx, event_rx) = broadcast::channel(100);

        // Generate a deterministic signing key from the coordinator ID
        let seed: [u8; 32] = {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(coordinator_id);
            hasher.update(b"coordinator_signing_key");
            hasher.finalize().into()
        };
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&seed);

        let coordinator = Self {
            config: config.clone(),
            nodes: Arc::new(RwLock::new(NodeRegistry::new())),
            scheduler: Arc::new(RwLock::new(TaskScheduler::new(config.scheduler))),
            verifier: Arc::new(RwLock::new(ExecutionVerifier::new(config.verifier))),
            event_tx,
            message_rx,
            message_tx: message_tx.clone(),
            coordinator_id,
            signing_key,
            metrics: Arc::new(RwLock::new(CoordinatorMetrics::default())),
            running: Arc::new(RwLock::new(false)),
            current_epoch: Arc::new(AtomicU64::new(0)),
            result_store: Arc::new(RwLock::new(HashMap::new())),
            completed_counter: Arc::new(AtomicU64::new(0)),
            cumulative_latency_ms: Arc::new(AtomicU64::new(0)),
            rewards_counter: Arc::new(AtomicU64::new(0)),
        };

        (coordinator, message_tx, event_rx)
    }

    /// Sign a message with the coordinator's Ed25519 key
    fn sign_message(&self, message: &[u8]) -> Signature {
        use ed25519_dalek::Signer;
        let sig = self.signing_key.sign(message);
        Signature(sig.to_bytes())
    }

    fn node_id_hex(node_id: &NodeId) -> String {
        hex::encode(node_id)
    }

    /// Get the current epoch
    fn epoch(&self) -> u64 {
        self.current_epoch.load(Ordering::Relaxed)
    }

    /// Advance to the next epoch
    fn advance_epoch(&self) -> u64 {
        self.current_epoch.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Start the coordinator
    pub async fn start(&mut self) -> SwarmResult<()> {
        {
            let mut running = self.running.write().await;
            *running = true;
        }

        // Start background tasks
        self.spawn_heartbeat_monitor();
        self.spawn_task_scheduler();
        self.spawn_timeout_checker();
        self.spawn_metrics_updater();
        self.spawn_epoch_advancer();

        // Main message processing loop
        self.run_message_loop().await
    }

    /// Stop the coordinator
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
    }

    /// Process incoming messages
    async fn run_message_loop(&mut self) -> SwarmResult<()> {
        while *self.running.read().await {
            match tokio::time::timeout(std::time::Duration::from_secs(1), self.message_rx.recv())
                .await
            {
                Ok(Some(envelope)) => {
                    if let Err(e) = self.handle_message(envelope).await {
                        tracing::warn!("Error handling message: {:?}", e);
                    }
                }
                Ok(None) => break,  // Channel closed
                Err(_) => continue, // Timeout, check running flag
            }
        }
        Ok(())
    }

    /// Handle an incoming message
    async fn handle_message(&self, envelope: MessageEnvelope) -> SwarmResult<()> {
        match envelope.message {
            SwarmMessage::JoinRequest(req) => self.handle_join_request(req).await,
            SwarmMessage::LeaveNotification(notif) => self.handle_leave(notif).await,
            SwarmMessage::Heartbeat(hb) => self.handle_heartbeat(hb).await,
            SwarmMessage::TaskSubmission(sub) => self.handle_task_submission(sub).await,
            SwarmMessage::TaskStarted(started) => self.handle_task_started(started).await,
            SwarmMessage::TaskResult(result) => self.handle_task_result(result).await,
            SwarmMessage::VerificationResult(ver) => self.handle_verification_result(ver).await,
            _ => Ok(()), // Ignore other message types
        }
    }

    /// Handle join request
    async fn handle_join_request(&self, req: JoinRequest) -> SwarmResult<()> {
        // Verify minimum stake
        if req.stake < self.config.min_stake {
            // Sign rejection message with coordinator key
            let reject_msg = format!("join_rejected:{}", Self::node_id_hex(&req.node_id));
            let response = JoinResponse {
                accepted: false,
                reason: Some(format!(
                    "Insufficient stake: {} required, {} provided",
                    self.config.min_stake, req.stake
                )),
                bootstrap_peers: Vec::new(),
                current_epoch: self.epoch(),
                signature: self.sign_message(reject_msg.as_bytes()),
            };
            self.send_response(req.node_id, SwarmMessage::JoinResponse(response))
                .await?;
            return Ok(());
        }

        // Create node
        let node = SwarmNode {
            id: req.node_id,
            peer_address: req.peer_address,
            region: req.region.clone(),
            gpu: req.gpu_capabilities,
            status: NodeStatus::Online,
            metrics: Default::default(),
            stake: req.stake,
            supported_tasks: req.supported_tasks,
            version: req.version,
            registered_at: chrono::Utc::now().timestamp(),
        };

        // Register node
        {
            let mut nodes = self.nodes.write().await;
            nodes.register(node)?;
        }

        // Get bootstrap peers
        let bootstrap_peers = {
            let nodes = self.nodes.read().await;
            nodes
                .online_nodes()
                .iter()
                .take(10)
                .map(|n| n.peer_address.clone())
                .collect()
        };

        // Sign acceptance message with coordinator key
        let accept_msg = format!(
            "join_accepted:{}:epoch:{}",
            Self::node_id_hex(&req.node_id),
            self.epoch()
        );
        let response = JoinResponse {
            accepted: true,
            reason: None,
            bootstrap_peers,
            current_epoch: self.epoch(),
            signature: self.sign_message(accept_msg.as_bytes()),
        };
        self.send_response(req.node_id, SwarmMessage::JoinResponse(response))
            .await?;

        // Emit event
        self.emit_event(CoordinatorEvent::NodeJoined {
            node_id: req.node_id,
            region: req.region,
        });

        Ok(())
    }

    /// Handle leave notification
    async fn handle_leave(&self, notif: LeaveNotification) -> SwarmResult<()> {
        {
            let mut nodes = self.nodes.write().await;
            nodes.unregister(&notif.node_id);
        }

        self.emit_event(CoordinatorEvent::NodeLeft {
            node_id: notif.node_id,
            reason: notif.reason,
        });

        Ok(())
    }

    /// Handle heartbeat
    async fn handle_heartbeat(&self, hb: Heartbeat) -> SwarmResult<()> {
        let mut nodes = self.nodes.write().await;

        if let Some(node) = nodes.get_mut(&hb.node_id) {
            node.heartbeat();
            node.metrics = hb.metrics;
            node.gpu.available_vram = hb.available_vram;
        }

        // Get pending tasks for this node from the scheduler
        let pending_tasks = {
            let scheduler = self.scheduler.read().await;
            scheduler.pending_for_node(&hb.node_id).unwrap_or_default()
        };

        let ack = HeartbeatAck {
            timestamp: hb.timestamp,
            pending_tasks,
        };

        drop(nodes);

        self.send_response(hb.node_id, SwarmMessage::HeartbeatAck(ack))
            .await
    }

    /// Handle task submission
    async fn handle_task_submission(&self, sub: TaskSubmission) -> SwarmResult<()> {
        let task_id = sub.task.id;
        let priority = sub.task.priority as u8;

        // Add to scheduler
        {
            let mut scheduler = self.scheduler.write().await;
            scheduler.submit(sub.task)?;
        }

        self.emit_event(CoordinatorEvent::TaskSubmitted { task_id, priority });

        // Trigger scheduling
        self.schedule_tasks().await?;

        Ok(())
    }

    /// Handle task started notification
    async fn handle_task_started(&self, started: TaskStarted) -> SwarmResult<()> {
        let mut scheduler = self.scheduler.write().await;
        scheduler.mark_started(started.task_id)?;
        Ok(())
    }

    /// Handle task result
    async fn handle_task_result(&self, result: TaskResult) -> SwarmResult<()> {
        let task_id = result.task_id;
        let execution_time_ms = result.execution_time_ms;
        let executor_id = result.executor;

        if result.success {
            // Start verification with selected verifiers
            let verifiers = self.select_verifiers(&result.executor, 2).await?;

            // Get original task from scheduler for verification
            let original_task = {
                let scheduler = self.scheduler.read().await;
                scheduler.get_task(task_id).cloned()
            };

            if let Some(task) = original_task {
                if !verifiers.is_empty() {
                    // Start verification process
                    let mut verifier = self.verifier.write().await;
                    if let Err(e) =
                        verifier.start_verification(task, result.clone(), verifiers.clone())
                    {
                        tracing::warn!(
                            "Could not start verification for task {:?}: {:?}",
                            task_id,
                            e
                        );
                    }
                }
            }

            // Store the result
            {
                let mut store = self.result_store.write().await;
                store.insert(task_id, result.clone());
            }

            {
                let mut scheduler = self.scheduler.write().await;
                scheduler.mark_completed(task_id, result.result_hash, result.compute_units)?;
            }

            // Track metrics
            self.completed_counter.fetch_add(1, Ordering::Relaxed);
            if execution_time_ms > 0 {
                self.cumulative_latency_ms
                    .fetch_add(execution_time_ms, Ordering::Relaxed);
            }

            self.emit_event(CoordinatorEvent::TaskCompleted {
                task_id,
                success: true,
            });

            let reward = self
                .calculate_task_reward(task_id, result.compute_units)
                .await;
            self.apply_reward(executor_id, reward).await?;
        } else {
            let mut scheduler = self.scheduler.write().await;
            scheduler.mark_failed(task_id, result.error.unwrap_or_default())?;

            self.emit_event(CoordinatorEvent::TaskCompleted {
                task_id,
                success: false,
            });

            self.apply_slash(
                executor_id,
                self.config.failure_slash_percentage,
                "Execution failure",
            )
            .await?;
        }

        Ok(())
    }

    /// Handle verification result
    async fn handle_verification_result(&self, ver: VerificationResult) -> SwarmResult<()> {
        let mut verifier = self.verifier.write().await;

        if let Some(summary) = verifier.submit_verification(ver.task_id, ver)? {
            self.emit_event(CoordinatorEvent::VerificationCompleted {
                task_id: summary.task_id,
                verdict: summary.verdict,
            });

            // Handle slashing for invalid results
            if summary.verdict == Verdict::Invalid {
                self.slash_for_invalid_result(&summary).await?;
            }
        }

        Ok(())
    }

    /// Schedule pending tasks
    async fn schedule_tasks(&self) -> SwarmResult<()> {
        // Collect all assignments first
        let assignments = {
            let nodes = self.nodes.read().await;
            let mut scheduler = self.scheduler.write().await;
            scheduler.schedule_batch(&nodes)
        };

        // Process assignments outside of lock scope
        for (task_id, node_id) in assignments {
            {
                let mut scheduler = self.scheduler.write().await;
                scheduler.assign(task_id, node_id)?;
            }

            self.emit_event(CoordinatorEvent::TaskAssigned { task_id, node_id });

            // Select verifiers for this task assignment
            let verifiers = self.select_verifiers(&node_id, 2).await.unwrap_or_default();

            // Sign the task assignment
            let assign_msg = format!(
                "assign:{}:{}:{}",
                task_id,
                Self::node_id_hex(&node_id),
                self.epoch()
            );
            let assignment = TaskAssignment {
                task_id,
                primary_executor: node_id,
                verifiers,
                assigned_at: chrono::Utc::now().timestamp(),
                signature: self.sign_message(assign_msg.as_bytes()),
            };

            self.send_response(node_id, SwarmMessage::TaskAssignment(assignment))
                .await?;
        }

        Ok(())
    }

    /// Select verifier nodes
    async fn select_verifiers(&self, executor: &NodeId, count: usize) -> SwarmResult<Vec<NodeId>> {
        let nodes = self.nodes.read().await;

        let verifiers: Vec<_> = nodes
            .online_nodes()
            .iter()
            .filter(|n| n.id != *executor && n.metrics.reputation >= 5000)
            .take(count)
            .map(|n| n.id)
            .collect();

        Ok(verifiers)
    }

    async fn calculate_task_reward(&self, task_id: TaskId, compute_units: u64) -> u64 {
        let base_reward = {
            let scheduler = self.scheduler.read().await;
            scheduler
                .get_task(task_id)
                .map(|t| t.reward)
                .unwrap_or(self.config.min_task_reward)
        };
        let cu_bonus = (compute_units / 1000) * self.config.reward_per_1000_cu;
        base_reward.max(self.config.min_task_reward) + cu_bonus
    }

    async fn apply_reward(&self, node_id: NodeId, amount: u64) -> SwarmResult<()> {
        if amount == 0 {
            return Ok(());
        }

        let mut nodes = self.nodes.write().await;
        if let Some(node) = nodes.get_mut(&node_id) {
            node.metrics.total_rewards = node.metrics.total_rewards.saturating_add(amount);
            node.stake = node.stake.saturating_add(amount / 10);
            self.rewards_counter.fetch_add(amount, Ordering::Relaxed);
        }
        Ok(())
    }

    async fn apply_slash(&self, node_id: NodeId, percent: u8, reason: &str) -> SwarmResult<()> {
        let mut nodes = self.nodes.write().await;
        if let Some(node) = nodes.get_mut(&node_id) {
            let slash_amount = (node.stake.saturating_mul(percent as u64)) / 100;
            node.stake = node.stake.saturating_sub(slash_amount);
            node.metrics.reputation = node.metrics.reputation.saturating_sub(250);
            self.emit_event(CoordinatorEvent::NodeSlashed {
                node_id,
                amount: slash_amount,
                reason: reason.to_string(),
            });
        }
        Ok(())
    }

    /// Slash a node for invalid result
    async fn slash_for_invalid_result(&self, summary: &VerificationSummary) -> SwarmResult<()> {
        for node_id in &summary.invalid_voters {
            self.apply_slash(
                *node_id,
                self.config.slash_percentage,
                "Invalid execution result",
            )
            .await?;
        }

        Ok(())
    }

    /// Send a response to a node
    async fn send_response(&self, _target: NodeId, message: SwarmMessage) -> SwarmResult<()> {
        let envelope = MessageEnvelope::new(self.coordinator_id, message);

        // In practice, this would send via the network layer
        tracing::debug!("Sending message: {:?}", envelope);

        Ok(())
    }

    /// Emit an event
    fn emit_event(&self, event: CoordinatorEvent) {
        let _ = self.event_tx.send(event);
    }

    /// Spawn heartbeat monitor task
    fn spawn_heartbeat_monitor(&self) {
        let nodes = Arc::clone(&self.nodes);
        let timeout = self.config.heartbeat_timeout_secs;
        let running = Arc::clone(&self.running);
        let event_tx = self.event_tx.clone();

        tokio::spawn(async move {
            while *running.read().await {
                tokio::time::sleep(std::time::Duration::from_secs(10)).await;

                let mut nodes_guard = nodes.write().await;
                let stale: Vec<_> = nodes_guard
                    .online_nodes()
                    .iter()
                    .filter(|n| n.is_stale(timeout as i64))
                    .map(|n| n.id)
                    .collect();

                for node_id in stale {
                    nodes_guard
                        .update_status(&node_id, NodeStatus::Offline)
                        .ok();
                    let _ = event_tx.send(CoordinatorEvent::NodeLeft {
                        node_id,
                        reason: "Heartbeat timeout".to_string(),
                    });
                }
            }
        });
    }

    /// Spawn task scheduler task
    fn spawn_task_scheduler(&self) {
        // Scheduling happens on-demand in this implementation
    }

    /// Spawn timeout checker task
    fn spawn_timeout_checker(&self) {
        let scheduler = Arc::clone(&self.scheduler);
        let verifier = Arc::clone(&self.verifier);
        let running = Arc::clone(&self.running);
        let event_tx = self.event_tx.clone();
        let nodes = Arc::clone(&self.nodes);
        let timeout_slash_percentage = self.config.timeout_slash_percentage;

        tokio::spawn(async move {
            while *running.read().await {
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;

                // Check scheduler timeouts
                {
                    let mut sched = scheduler.write().await;
                    let timed_out = sched.check_timeouts_with_executors();
                    for (task_id, maybe_node_id) in timed_out {
                        if let Some(node_id) = maybe_node_id {
                            let mut nodes_guard = nodes.write().await;
                            if let Some(node) = nodes_guard.get_mut(&node_id) {
                                let slash =
                                    (node.stake.saturating_mul(timeout_slash_percentage as u64))
                                        / 100;
                                node.stake = node.stake.saturating_sub(slash);
                                node.metrics.reputation =
                                    node.metrics.reputation.saturating_sub(500);
                                let _ = event_tx.send(CoordinatorEvent::NodeSlashed {
                                    node_id,
                                    amount: slash,
                                    reason: "Task timeout".to_string(),
                                });
                            }
                        }
                        let _ = event_tx.send(CoordinatorEvent::TaskCompleted {
                            task_id,
                            success: false,
                        });
                    }
                }

                // Check verification timeouts
                {
                    let mut ver = verifier.write().await;
                    let timed_out = ver.check_timeouts();
                    for summary in timed_out {
                        let _ = event_tx.send(CoordinatorEvent::VerificationCompleted {
                            task_id: summary.task_id,
                            verdict: summary.verdict,
                        });
                    }
                }
            }
        });
    }

    /// Spawn epoch advancer - advances epoch every 60 seconds
    fn spawn_epoch_advancer(&self) {
        let epoch = Arc::clone(&self.current_epoch);
        let running = Arc::clone(&self.running);

        tokio::spawn(async move {
            while *running.read().await {
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                let new_epoch = epoch.fetch_add(1, Ordering::SeqCst) + 1;
                tracing::info!("Epoch advanced to {}", new_epoch);
            }
        });
    }

    /// Spawn metrics updater task
    fn spawn_metrics_updater(&self) {
        let nodes = Arc::clone(&self.nodes);
        let scheduler = Arc::clone(&self.scheduler);
        let metrics = Arc::clone(&self.metrics);
        let running = Arc::clone(&self.running);
        let event_tx = self.event_tx.clone();
        let completed_counter = Arc::clone(&self.completed_counter);
        let cumulative_latency = Arc::clone(&self.cumulative_latency_ms);
        let rewards_counter = Arc::clone(&self.rewards_counter);

        tokio::spawn(async move {
            while *running.read().await {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;

                let nodes_guard = nodes.read().await;
                let sched_guard = scheduler.read().await;
                let stats = sched_guard.stats();

                let online_count = nodes_guard.online_nodes().len();
                let total_capacity = nodes_guard.total_compute_capacity();
                let status_counts = nodes_guard.count_by_status();

                // Calculate real metrics from atomic counters
                let completed_hour = completed_counter.load(Ordering::Relaxed);
                let total_latency = cumulative_latency.load(Ordering::Relaxed);
                let avg_latency = if completed_hour > 0 {
                    total_latency / completed_hour
                } else {
                    0
                };
                let rewards_hour = rewards_counter.load(Ordering::Relaxed);

                let new_metrics = CoordinatorMetrics {
                    total_nodes: status_counts.values().sum(),
                    online_nodes: online_count,
                    queued_tasks: stats.pending_count,
                    executing_tasks: stats.executing_count,
                    completed_tasks_hour: completed_hour,
                    total_compute_capacity: total_capacity,
                    avg_task_latency_ms: avg_latency,
                    rewards_distributed_hour: rewards_hour,
                };

                {
                    let mut m = metrics.write().await;
                    *m = new_metrics.clone();
                }

                let _ = event_tx.send(CoordinatorEvent::MetricsUpdated(new_metrics));
            }
        });
    }

    /// Submit a task to the coordinator
    pub async fn submit_task(&self, task: Task) -> SwarmResult<()> {
        let task_id = task.id;
        let priority = task.priority as u8;

        {
            let mut scheduler = self.scheduler.write().await;
            scheduler.submit(task)?;
        }

        self.emit_event(CoordinatorEvent::TaskSubmitted { task_id, priority });
        self.schedule_tasks().await?;

        Ok(())
    }

    /// Get the status of a task
    pub async fn get_task_status(&self, task_id: &TaskId) -> Option<TaskStatus> {
        let scheduler = self.scheduler.read().await;
        scheduler.get_status(*task_id)
    }

    /// Get the result of a completed task from the result store.
    pub async fn get_task_result(&self, task_id: &TaskId) -> Option<TaskResult> {
        let store = self.result_store.read().await;
        store.get(task_id).cloned()
    }

    /// Cancel a task
    pub async fn cancel_task(&self, task_id: &TaskId) -> SwarmResult<bool> {
        let mut scheduler = self.scheduler.write().await;
        match scheduler.cancel(*task_id) {
            Ok(()) => Ok(true),
            Err(SwarmError::TaskNotFound(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Get current metrics
    pub async fn metrics(&self) -> CoordinatorMetrics {
        self.metrics.read().await.clone()
    }

    /// Get scheduler stats
    pub async fn scheduler_stats(&self) -> SchedulerStats {
        self.scheduler.read().await.stats()
    }
}
