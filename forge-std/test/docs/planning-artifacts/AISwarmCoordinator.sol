// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";
import "@openzeppelin/contracts-upgradeable/proxy/utils/UUPSUpgradeable.sol";
import "@openzeppelin/contracts-upgradeable/access/AccessControlUpgradeable.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";

/**
 * @title AISwarmCoordinator
 * @notice On-chain coordination for off-chain AI agent swarm
 * @dev Manages tasks, rewards, and reputation for distributed AI agents
 *
 * Agent Types:
 * - ARBITRAGE: Cross-chain/DEX arbitrage detection
 * - LENDING: Yield optimization and health monitoring
 * - LP_REBALANCE: Liquidity position management
 * - PREDICTION: Market prediction and analytics
 * - RISK: Risk assessment and alerts
 * - CONTENT: NFT metadata, reports generation
 */
contract AISwarmCoordinator is
    Initializable,
    UUPSUpgradeable,
    AccessControlUpgradeable,
    ReentrancyGuard
{
    using SafeERC20 for IERC20;

    // ============ Constants ============

    bytes32 public constant COORDINATOR_ROLE = keccak256("COORDINATOR_ROLE");
    bytes32 public constant VALIDATOR_ROLE = keccak256("VALIDATOR_ROLE");
    bytes32 public constant UPGRADER_ROLE = keccak256("UPGRADER_ROLE");

    uint256 public constant BPS_PRECISION = 10000;
    uint256 public constant MIN_STAKE = 100e18; // 100 tokens
    uint256 public constant MAX_TASK_DURATION = 7 days;
    uint256 public constant REPUTATION_PRECISION = 1000;

    // ============ Enums ============

    enum AgentType {
        ARBITRAGE,
        LENDING,
        LP_REBALANCE,
        PREDICTION,
        RISK,
        CONTENT,
        GENERAL
    }

    enum TaskPriority {
        LOW,
        MEDIUM,
        HIGH,
        CRITICAL
    }

    enum TaskStatus {
        CREATED,
        ASSIGNED,
        IN_PROGRESS,
        SUBMITTED,
        VALIDATED,
        COMPLETED,
        FAILED,
        DISPUTED,
        CANCELLED
    }

    enum RewardType {
        FIXED,
        PERCENTAGE,
        PERFORMANCE_BASED,
        BOUNTY
    }

    // ============ Structs ============

    struct Agent {
        address owner;
        AgentType agentType;
        bool active;
        uint256 stake;
        uint256 reputation;
        uint256 tasksCompleted;
        uint256 tasksFailed;
        uint256 totalEarnings;
        uint256 registeredAt;
        uint256 lastActive;
        string endpoint; // Off-chain API endpoint
        bytes32[] specializations;
    }

    struct Task {
        uint256 taskId;
        bytes32 taskHash;
        address creator;
        AgentType requiredType;
        TaskPriority priority;
        TaskStatus status;
        address assignedAgent;
        uint256 reward;
        RewardType rewardType;
        uint256 deadline;
        uint256 createdAt;
        uint256 completedAt;
        bytes inputData;
        bytes outputData;
        bytes32 resultHash;
        uint256 validationScore;
    }

    struct TaskResult {
        uint256 taskId;
        address agent;
        bytes32 resultHash;
        bytes resultData;
        uint256 confidence;
        uint256 submittedAt;
        uint256 gasUsed;
        bool validated;
        int256 profitGenerated;
    }

    struct SwarmMetrics {
        uint256 totalAgents;
        uint256 activeAgents;
        uint256 totalTasks;
        uint256 completedTasks;
        uint256 totalRewardsPaid;
        uint256 averageTaskTime;
        uint256 averageValidationScore;
    }

    struct AgentStats {
        uint256 totalTasks;
        uint256 successRate;
        uint256 averageResponseTime;
        uint256 averageConfidence;
        int256 totalProfitGenerated;
    }

    // ============ State Variables ============

    // Agents
    mapping(address => Agent) public agents;
    address[] public agentList;
    mapping(AgentType => address[]) public agentsByType;

    // Tasks
    mapping(uint256 => Task) public tasks;
    uint256 public taskCount;
    mapping(uint256 => TaskResult) public taskResults;

    // Task queues by priority
    mapping(TaskPriority => uint256[]) public taskQueues;

    // Agent task history
    mapping(address => uint256[]) public agentTaskHistory;

    // Staking token
    IERC20 public stakingToken;

    // Reward token (can be same as staking)
    IERC20 public rewardToken;

    // Treasury for rewards
    address public treasury;

    // Global metrics
    SwarmMetrics public swarmMetrics;

    // Minimum reputation for different task types
    mapping(TaskPriority => uint256) public minReputationForPriority;

    // ============ Events ============

    event AgentRegistered(
        address indexed agent,
        AgentType agentType,
        uint256 stake
    );

    event AgentDeactivated(address indexed agent, string reason);

    event TaskCreated(
        uint256 indexed taskId,
        AgentType requiredType,
        TaskPriority priority,
        uint256 reward
    );

    event TaskAssigned(uint256 indexed taskId, address indexed agent);

    event TaskSubmitted(
        uint256 indexed taskId,
        address indexed agent,
        bytes32 resultHash
    );

    event TaskValidated(
        uint256 indexed taskId,
        uint256 validationScore,
        bool passed
    );

    event TaskCompleted(
        uint256 indexed taskId,
        address indexed agent,
        uint256 reward
    );

    event RewardPaid(address indexed agent, uint256 amount, uint256 taskId);

    event ReputationUpdated(
        address indexed agent,
        uint256 oldReputation,
        uint256 newReputation
    );

    event StakeSlashed(address indexed agent, uint256 amount, string reason);

    // ============ Errors ============

    error AgentNotRegistered();
    error AgentNotActive();
    error InsufficientStake();
    error InsufficientReputation();
    error TaskNotFound();
    error TaskNotAssignable();
    error TaskNotSubmittable();
    error UnauthorizedAgent();
    error DeadlineExceeded();
    error AlreadySubmitted();

    // ============ Initializer ============

    /// @custom:oz-upgrades-unsafe-allow constructor
    constructor() {
        _disableInitializers();
    }

    function initialize(
        address _admin,
        address _stakingToken,
        address _rewardToken,
        address _treasury
    ) external initializer {
        __AccessControl_init();

        _grantRole(DEFAULT_ADMIN_ROLE, _admin);
        _grantRole(COORDINATOR_ROLE, _admin);
        _grantRole(VALIDATOR_ROLE, _admin);
        _grantRole(UPGRADER_ROLE, _admin);

        stakingToken = IERC20(_stakingToken);
        rewardToken = IERC20(_rewardToken);
        treasury = _treasury;

        // Set minimum reputation for task priorities
        minReputationForPriority[TaskPriority.LOW] = 0;
        minReputationForPriority[TaskPriority.MEDIUM] = 300;
        minReputationForPriority[TaskPriority.HIGH] = 600;
        minReputationForPriority[TaskPriority.CRITICAL] = 800;
    }

    // ============ Agent Management ============

    /**
     * @notice Register as an AI agent
     */
    function registerAgent(
        AgentType agentType,
        string calldata endpoint,
        bytes32[] calldata specializations
    ) external nonReentrant {
        require(agents[msg.sender].owner == address(0), "Already registered");

        // Require stake
        stakingToken.safeTransferFrom(msg.sender, address(this), MIN_STAKE);

        agents[msg.sender] = Agent({
            owner: msg.sender,
            agentType: agentType,
            active: true,
            stake: MIN_STAKE,
            reputation: 500, // Start at 50%
            tasksCompleted: 0,
            tasksFailed: 0,
            totalEarnings: 0,
            registeredAt: block.timestamp,
            lastActive: block.timestamp,
            endpoint: endpoint,
            specializations: specializations
        });

        agentList.push(msg.sender);
        agentsByType[agentType].push(msg.sender);
        swarmMetrics.totalAgents++;
        swarmMetrics.activeAgents++;

        emit AgentRegistered(msg.sender, agentType, MIN_STAKE);
    }

    /**
     * @notice Add stake to increase capacity
     */
    function addStake(uint256 amount) external nonReentrant {
        Agent storage agent = agents[msg.sender];
        if (agent.owner == address(0)) revert AgentNotRegistered();

        stakingToken.safeTransferFrom(msg.sender, address(this), amount);
        agent.stake += amount;
    }

    /**
     * @notice Withdraw excess stake
     */
    function withdrawStake(uint256 amount) external nonReentrant {
        Agent storage agent = agents[msg.sender];
        if (agent.owner == address(0)) revert AgentNotRegistered();

        uint256 maxWithdrawable = agent.stake > MIN_STAKE
            ? agent.stake - MIN_STAKE
            : 0;
        require(amount <= maxWithdrawable, "Insufficient withdrawable stake");

        agent.stake -= amount;
        stakingToken.safeTransfer(msg.sender, amount);
    }

    /**
     * @notice Deactivate agent
     */
    function deactivateAgent(string calldata reason) external {
        Agent storage agent = agents[msg.sender];
        if (agent.owner == address(0)) revert AgentNotRegistered();

        agent.active = false;
        swarmMetrics.activeAgents--;

        emit AgentDeactivated(msg.sender, reason);
    }

    /**
     * @notice Update agent endpoint
     */
    function updateEndpoint(string calldata newEndpoint) external {
        Agent storage agent = agents[msg.sender];
        if (agent.owner == address(0)) revert AgentNotRegistered();

        agent.endpoint = newEndpoint;
    }

    // ============ Task Management ============

    /**
     * @notice Create a new task
     */
    function createTask(
        AgentType requiredType,
        TaskPriority priority,
        uint256 reward,
        RewardType rewardType,
        uint256 deadline,
        bytes calldata inputData
    ) external onlyRole(COORDINATOR_ROLE) returns (uint256 taskId) {
        require(deadline > block.timestamp, "Invalid deadline");
        require(
            deadline <= block.timestamp + MAX_TASK_DURATION,
            "Deadline too far"
        );

        taskId = ++taskCount;

        tasks[taskId] = Task({
            taskId: taskId,
            taskHash: keccak256(inputData),
            creator: msg.sender,
            requiredType: requiredType,
            priority: priority,
            status: TaskStatus.CREATED,
            assignedAgent: address(0),
            reward: reward,
            rewardType: rewardType,
            deadline: deadline,
            createdAt: block.timestamp,
            completedAt: 0,
            inputData: inputData,
            outputData: "",
            resultHash: bytes32(0),
            validationScore: 0
        });

        // Add to priority queue
        taskQueues[priority].push(taskId);
        swarmMetrics.totalTasks++;

        emit TaskCreated(taskId, requiredType, priority, reward);
    }

    /**
     * @notice Batch create tasks
     */
    function batchCreateTasks(
        AgentType[] calldata types,
        TaskPriority[] calldata priorities,
        uint256[] calldata rewards,
        RewardType[] calldata rewardTypes,
        uint256[] calldata deadlines,
        bytes[] calldata inputDatas
    ) external onlyRole(COORDINATOR_ROLE) returns (uint256[] memory taskIds) {
        require(types.length == priorities.length, "Length mismatch");

        taskIds = new uint256[](types.length);

        for (uint256 i = 0; i < types.length; i++) {
            taskIds[i] = this.createTask(
                types[i],
                priorities[i],
                rewards[i],
                rewardTypes[i],
                deadlines[i],
                inputDatas[i]
            );
        }
    }

    /**
     * @notice Claim a task
     */
    function claimTask(uint256 taskId) external nonReentrant {
        Task storage task = tasks[taskId];
        if (task.taskId == 0) revert TaskNotFound();
        if (task.status != TaskStatus.CREATED) revert TaskNotAssignable();
        if (block.timestamp > task.deadline) revert DeadlineExceeded();

        Agent storage agent = agents[msg.sender];
        if (agent.owner == address(0)) revert AgentNotRegistered();
        if (!agent.active) revert AgentNotActive();
        if (
            agent.agentType != task.requiredType &&
            task.requiredType != AgentType.GENERAL
        ) {
            revert UnauthorizedAgent();
        }
        if (agent.reputation < minReputationForPriority[task.priority]) {
            revert InsufficientReputation();
        }

        task.status = TaskStatus.ASSIGNED;
        task.assignedAgent = msg.sender;
        agent.lastActive = block.timestamp;

        emit TaskAssigned(taskId, msg.sender);
    }

    /**
     * @notice Submit task result
     */
    function submitResult(
        uint256 taskId,
        bytes32 resultHash,
        bytes calldata resultData,
        uint256 confidence,
        int256 profitGenerated
    ) external nonReentrant {
        Task storage task = tasks[taskId];
        if (task.taskId == 0) revert TaskNotFound();
        if (task.assignedAgent != msg.sender) revert UnauthorizedAgent();
        if (
            task.status != TaskStatus.ASSIGNED &&
            task.status != TaskStatus.IN_PROGRESS
        ) {
            revert TaskNotSubmittable();
        }
        if (block.timestamp > task.deadline) revert DeadlineExceeded();

        task.status = TaskStatus.SUBMITTED;
        task.resultHash = resultHash;
        task.outputData = resultData;

        taskResults[taskId] = TaskResult({
            taskId: taskId,
            agent: msg.sender,
            resultHash: resultHash,
            resultData: resultData,
            confidence: confidence,
            submittedAt: block.timestamp,
            gasUsed: 0, // Set by validator
            validated: false,
            profitGenerated: profitGenerated
        });

        agents[msg.sender].lastActive = block.timestamp;

        emit TaskSubmitted(taskId, msg.sender, resultHash);
    }

    /**
     * @notice Validate task result
     */
    function validateResult(
        uint256 taskId,
        uint256 validationScore,
        bool passed
    ) external onlyRole(VALIDATOR_ROLE) {
        Task storage task = tasks[taskId];
        if (task.taskId == 0) revert TaskNotFound();
        if (task.status != TaskStatus.SUBMITTED) revert TaskNotSubmittable();

        TaskResult storage result = taskResults[taskId];
        result.validated = true;

        task.validationScore = validationScore;

        if (passed && validationScore >= 500) {
            // 50% threshold
            task.status = TaskStatus.VALIDATED;
            emit TaskValidated(taskId, validationScore, true);
        } else {
            task.status = TaskStatus.FAILED;
            emit TaskValidated(taskId, validationScore, false);
        }
    }

    /**
     * @notice Complete task and pay reward
     */
    function completeTask(
        uint256 taskId
    ) external nonReentrant onlyRole(COORDINATOR_ROLE) {
        Task storage task = tasks[taskId];
        if (task.taskId == 0) revert TaskNotFound();
        if (task.status != TaskStatus.VALIDATED) revert TaskNotSubmittable();

        Agent storage agent = agents[task.assignedAgent];

        // Calculate reward
        uint256 reward = _calculateReward(task);

        // Update task
        task.status = TaskStatus.COMPLETED;
        task.completedAt = block.timestamp;

        // Update agent
        agent.tasksCompleted++;
        agent.totalEarnings += reward;
        agentTaskHistory[task.assignedAgent].push(taskId);

        // Update reputation
        _updateReputation(agent, true, task.validationScore);

        // Pay reward
        rewardToken.safeTransferFrom(treasury, task.assignedAgent, reward);

        // Update metrics
        swarmMetrics.completedTasks++;
        swarmMetrics.totalRewardsPaid += reward;

        emit TaskCompleted(taskId, task.assignedAgent, reward);
        emit RewardPaid(task.assignedAgent, reward, taskId);
    }

    /**
     * @notice Handle failed task
     */
    function handleFailedTask(
        uint256 taskId,
        bool slashStake,
        string calldata reason
    ) external onlyRole(COORDINATOR_ROLE) {
        Task storage task = tasks[taskId];
        if (task.taskId == 0) revert TaskNotFound();

        Agent storage agent = agents[task.assignedAgent];
        agent.tasksFailed++;

        // Update reputation negatively
        _updateReputation(agent, false, task.validationScore);

        // Optionally slash stake
        if (slashStake && agent.stake > 0) {
            uint256 slashAmount = agent.stake / 10; // 10% slash
            agent.stake -= slashAmount;
            stakingToken.safeTransfer(treasury, slashAmount);

            emit StakeSlashed(task.assignedAgent, slashAmount, reason);
        }

        task.status = TaskStatus.FAILED;
    }

    // ============ View Functions ============

    /**
     * @notice Get agent info
     */
    function getAgent(address agentAddr) external view returns (Agent memory) {
        return agents[agentAddr];
    }

    /**
     * @notice Get agent statistics
     */
    function getAgentStats(
        address agentAddr
    ) external view returns (AgentStats memory) {
        Agent storage agent = agents[agentAddr];
        uint256[] storage history = agentTaskHistory[agentAddr];

        uint256 totalTasks = agent.tasksCompleted + agent.tasksFailed;
        uint256 successRate = totalTasks > 0
            ? (agent.tasksCompleted * BPS_PRECISION) / totalTasks
            : 0;

        int256 totalProfit = 0;
        uint256 totalConfidence = 0;
        uint256 totalTime = 0;

        for (uint256 i = 0; i < history.length && i < 100; i++) {
            TaskResult storage result = taskResults[history[i]];
            totalProfit += result.profitGenerated;
            totalConfidence += result.confidence;

            Task storage task = tasks[history[i]];
            if (task.completedAt > task.createdAt) {
                totalTime += task.completedAt - task.createdAt;
            }
        }

        uint256 avgConfidence = history.length > 0
            ? totalConfidence / history.length
            : 0;
        uint256 avgTime = history.length > 0 ? totalTime / history.length : 0;

        return
            AgentStats({
                totalTasks: totalTasks,
                successRate: successRate,
                averageResponseTime: avgTime,
                averageConfidence: avgConfidence,
                totalProfitGenerated: totalProfit
            });
    }

    /**
     * @notice Get task
     */
    function getTask(uint256 taskId) external view returns (Task memory) {
        return tasks[taskId];
    }

    /**
     * @notice Get task result
     */
    function getTaskResult(
        uint256 taskId
    ) external view returns (TaskResult memory) {
        return taskResults[taskId];
    }

    /**
     * @notice Get available tasks for an agent
     */
    function getAvailableTasks(
        address agentAddr,
        uint256 limit
    ) external view returns (uint256[] memory) {
        Agent storage agent = agents[agentAddr];
        if (agent.owner == address(0)) return new uint256[](0);

        uint256[] memory available = new uint256[](limit);
        uint256 count = 0;

        // Check all priorities from high to low
        for (
            uint256 p = uint256(TaskPriority.CRITICAL);
            p >= 0 && count < limit;
            p--
        ) {
            uint256[] storage queue = taskQueues[TaskPriority(p)];

            for (uint256 i = 0; i < queue.length && count < limit; i++) {
                Task storage task = tasks[queue[i]];

                if (
                    task.status == TaskStatus.CREATED &&
                    block.timestamp <= task.deadline &&
                    (task.requiredType == agent.agentType ||
                        task.requiredType == AgentType.GENERAL) &&
                    agent.reputation >= minReputationForPriority[task.priority]
                ) {
                    available[count++] = queue[i];
                }
            }

            if (p == 0) break;
        }

        // Resize array
        assembly {
            mstore(available, count)
        }

        return available;
    }

    /**
     * @notice Get swarm metrics
     */
    function getSwarmMetrics() external view returns (SwarmMetrics memory) {
        return swarmMetrics;
    }

    /**
     * @notice Get agents by type
     */
    function getAgentsByType(
        AgentType agentType
    ) external view returns (address[] memory) {
        return agentsByType[agentType];
    }

    // ============ Internal Functions ============

    function _calculateReward(
        Task storage task
    ) internal view returns (uint256) {
        if (task.rewardType == RewardType.FIXED) {
            return task.reward;
        } else if (task.rewardType == RewardType.PERFORMANCE_BASED) {
            // Scale by validation score
            return (task.reward * task.validationScore) / REPUTATION_PRECISION;
        } else if (task.rewardType == RewardType.PERCENTAGE) {
            // Reward is a percentage of profit generated
            TaskResult storage result = taskResults[task.taskId];
            if (result.profitGenerated > 0) {
                return
                    (uint256(result.profitGenerated) * task.reward) /
                    BPS_PRECISION;
            }
            return 0;
        }
        return task.reward;
    }

    function _updateReputation(
        Agent storage agent,
        bool success,
        uint256 score
    ) internal {
        uint256 oldReputation = agent.reputation;
        uint256 newReputation;

        if (success) {
            // Increase reputation based on score
            uint256 increase = (score * 10) / REPUTATION_PRECISION; // Max 10 points
            newReputation = agent.reputation + increase;
            if (newReputation > REPUTATION_PRECISION) {
                newReputation = REPUTATION_PRECISION;
            }
        } else {
            // Decrease reputation
            uint256 decrease = 50; // 5 points per failure
            if (agent.reputation > decrease) {
                newReputation = agent.reputation - decrease;
            } else {
                newReputation = 0;
            }
        }

        agent.reputation = newReputation;

        emit ReputationUpdated(agent.owner, oldReputation, newReputation);
    }

    function _authorizeUpgrade(
        address newImplementation
    ) internal override onlyRole(UPGRADER_ROLE) {}
}

            for (uint256 i = 0; i < queue.length && idx < count; i++) {
                Task storage task = tasks[queue[i]];

                if (
                    task.status == TaskStatus.CREATED &&
                    block.timestamp <= task.deadline &&
                    (task.requiredType == agent.agentType ||
                        task.requiredType == AgentType.GENERAL) &&
                    agent.reputation >= minReputationForPriority[task.priority]
                ) {
                    available[idx++] = queue[i];
                }
            }

            if (idx >= count) break;
        }

        return available;
    }

    /**
     * @notice Get swarm metrics
     */
    function getSwarmMetrics() external view returns (SwarmMetrics memory) {
        return swarmMetrics;
    }

    /**
     * @notice Get agents by type
     */
    function getAgentsByType(
        AgentType agentType
    ) external view returns (address[] memory) {
        return agentsByType[agentType];
    }

    // ============ Internal Functions ============

    function _calculateReward(
        Task storage task
    ) internal view returns (uint256) {
        if (task.rewardType == RewardType.FIXED) {
            return task.reward;
        } else if (task.rewardType == RewardType.PERFORMANCE_BASED) {
            // Scale by validation score
            return (task.reward * task.validationScore) / REPUTATION_PRECISION;
        } else if (task.rewardType == RewardType.PERCENTAGE) {
            // Reward is a percentage of profit generated
            TaskResult storage result = taskResults[task.taskId];
            if (result.profitGenerated > 0) {
                return
                    (uint256(result.profitGenerated) * task.reward) /
                    BPS_PRECISION;
            }
            return 0;
        }
        return task.reward;
    }

    function _updateReputation(
        Agent storage agent,
        bool success,
        uint256 score
    ) internal {
        uint256 oldReputation = agent.reputation;
        uint256 newReputation;

        if (success) {
            // Increase reputation based on score
            uint256 increase = (score * 10) / REPUTATION_PRECISION; // Max 10 points
            newReputation = agent.reputation + increase;
            if (newReputation > REPUTATION_PRECISION) {
                newReputation = REPUTATION_PRECISION;
            }
        } else {
            // Decrease reputation
            uint256 decrease = 50; // 5 points per failure
            if (agent.reputation > decrease) {
                newReputation = agent.reputation - decrease;
            } else {
                newReputation = 0;
            }
        }

        agent.reputation = newReputation;

        emit ReputationUpdated(agent.owner, oldReputation, newReputation);
    }

    function _authorizeUpgrade(
        address newImplementation
    ) internal override onlyRole(UPGRADER_ROLE) {}
}
