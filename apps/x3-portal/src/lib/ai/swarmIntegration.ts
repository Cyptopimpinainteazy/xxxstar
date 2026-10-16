import { callOpenRouter } from './openrouter';

export interface SwarmNode {
  id: string;
  type: 'gpu' | 'ai' | 'validator';
  status: 'active' | 'idle' | 'busy' | 'offline';
  capabilities: string[];
  load: number;
  coordinates: [number, number];
}

export interface SwarmTask {
  id: string;
  type: 'inference' | 'training' | 'validation' | 'prediction';
  priority: 'low' | 'medium' | 'high' | 'critical';
  data: any;
  model?: string;
  estimatedTime: number;
}

export interface SwarmResponse {
  taskId: string;
  result: any;
  nodeId: string;
  executionTime: number;
  model: string;
}

// AI Swarm Configuration
const SWARM_CONFIG = {
  coordinatorUrl: process.env.AI_SWARM_COORDINATOR_URL || 'http://localhost:8080',
  nodeUrl: process.env.AI_SWARM_NODE_URL || 'http://localhost:8081',
  enabled: process.env.AI_SWARM_ENABLED === 'true',
  maxRetries: 3,
  timeout: 30000,
};

// Free models optimized for different swarm tasks
const SWARM_MODELS = {
  inference: 'openrouter/auto',
  training: 'microsoft/phi-3-mini-128k-instruct',
  validation: 'meta-llama/llama-3.2-3b-instruct',
  prediction: 'google/gemini-pro-1.5',
  general: 'openai/gpt-3.5-turbo',
};

export class AISwarmIntegrator {
  private nodes: Map<string, SwarmNode> = new Map();
  private taskQueue: SwarmTask[] = [];
  private isInitialized = false;

  async initialize(): Promise<void> {
    if (!SWARM_CONFIG.enabled) {
      console.log('🤖 AI Swarm disabled in configuration');
      return;
    }

    try {
      // Initialize swarm connection
      await this.discoverNodes();
      this.isInitialized = true;
      console.log(`🤖 AI Swarm initialized with ${this.nodes.size} nodes`);
    } catch (error) {
      console.error('🤖 Failed to initialize AI Swarm:', error);
      // Continue without swarm - fallback to direct OpenRouter
    }
  }

  private async discoverNodes(): Promise<void> {
    try {
      const response = await fetch(`${SWARM_CONFIG.coordinatorUrl}/api/nodes`, {
        method: 'GET',
        headers: { 'Content-Type': 'application/json' },
        signal: AbortSignal.timeout(SWARM_CONFIG.timeout),
      });

      if (response.ok) {
        const nodes: SwarmNode[] = await response.json();
        nodes.forEach(node => this.nodes.set(node.id, node));
      }
    } catch (error) {
      console.log('🤖 Swarm coordinator not available, using direct OpenRouter');
    }
  }

  async processTask(task: SwarmTask): Promise<SwarmResponse> {
    const startTime = Date.now();
    const model = task.model || SWARM_MODELS[task.type] || SWARM_MODELS.general;

    try {
      // Try swarm processing first
      if (this.isInitialized && this.nodes.size > 0) {
        const swarmResponse = await this.processViaSwarm(task);
        if (swarmResponse) {
          return {
            ...swarmResponse,
            model,
            executionTime: Date.now() - startTime,
          };
        }
      }

      // Fallback to direct OpenRouter processing
      return await this.processViaOpenRouter(task, model, startTime);
    } catch (error) {
      console.error('🤖 Task processing failed:', error);
      throw new Error(`AI Swarm processing failed: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  }

  private async processViaSwarm(task: SwarmTask): Promise<SwarmResponse | null> {
    try {
      const suitableNodes = Array.from(this.nodes.values())
        .filter(node => 
          node.status === 'active' && 
          node.capabilities.includes('ai') &&
          node.load < 80
        );

      if (suitableNodes.length === 0) return null;

      const selectedNode = suitableNodes[0];
      
      const response = await fetch(`${SWARM_CONFIG.nodeUrl}/api/ai/process`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          taskId: task.id,
          taskType: task.type,
          data: task.data,
          model: task.model,
          priority: task.priority,
        }),
        signal: AbortSignal.timeout(SWARM_CONFIG.timeout),
      });

      if (response.ok) {
        return await response.json();
      }
    } catch (error) {
      console.log('🤖 Swarm processing failed, falling back to direct OpenRouter');
    }

    return null;
  }

  private async processViaOpenRouter(task: SwarmTask, model: string, startTime: number): Promise<SwarmResponse> {
    const apiKey = process.env.OPENROUTER_API_KEY;
    if (!apiKey) {
      throw new Error('OpenRouter API key not configured');
    }

    const messages = this.formatTaskAsMessages(task);
    
    const result = await callOpenRouter({
      apiKey,
      model,
      messages,
      maxTokens: 1000,
      temperature: 0.1,
    });

    return {
      taskId: task.id,
      result: result.content,
      nodeId: 'openrouter-direct',
      executionTime: Date.now() - startTime,
      model: result.model,
    };
  }

  private formatTaskAsMessages(task: SwarmTask): Array<{ role: 'system' | 'user' | 'assistant'; content: string }> {
    const { type, data, priority } = task;

    const systemPrompt = `You are an AI agent in the Atlas Sphere swarm network. 
    Task Type: ${type}
    Priority: ${priority}
    Process this task efficiently and provide accurate results.`;

    let userContent = '';
    
    switch (type) {
      case 'inference':
        userContent = `Perform inference on the following data: ${JSON.stringify(data)}`;
        break;
      case 'training':
        userContent = `Process training data: ${JSON.stringify(data)}`;
        break;
      case 'validation':
        userContent = `Validate the following: ${JSON.stringify(data)}`;
        break;
      case 'prediction':
        userContent = `Make predictions based on: ${JSON.stringify(data)}`;
        break;
      default:
        userContent = `Process task: ${JSON.stringify(data)}`;
    }

    return [
      { role: 'system', content: systemPrompt },
      { role: 'user', content: userContent },
    ];
  }

  async getSwarmStatus(): Promise<{
    enabled: boolean;
    nodes: number;
    activeNodes: number;
    totalLoad: number;
  }> {
    const activeNodes = Array.from(this.nodes.values())
      .filter(node => node.status === 'active').length;
    
    const totalLoad = Array.from(this.nodes.values())
      .reduce((sum, node) => sum + node.load, 0) / Math.max(this.nodes.size, 1);

    return {
      enabled: SWARM_CONFIG.enabled,
      nodes: this.nodes.size,
      activeNodes,
      totalLoad,
    };
  }

  async submitTask(task: Omit<SwarmTask, 'id'>): Promise<string> {
    const fullTask: SwarmTask = {
      ...task,
      id: `task-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
    };

    this.taskQueue.push(fullTask);
    
    // Process task asynchronously
    this.processTask(fullTask).catch(error => {
      console.error('🤖 Background task processing failed:', error);
    });

    return fullTask.id;
  }

  getAvailableModels(): string[] {
    return Object.values(SWARM_MODELS);
  }

  isSwarmEnabled(): boolean {
    return SWARM_CONFIG.enabled;
  }
}

// Singleton instance
export const aiSwarm = new AISwarmIntegrator();

// Initialize on module load
if (typeof window === 'undefined') {
  aiSwarm.initialize();
}
