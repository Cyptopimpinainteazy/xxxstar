/**
 * Template management module for X3 Foundry.
 *
 * Provides methods for discovering, registering, and forking DApp templates
 * from the Foundry template registry.
 *
 * @module @x3/foundry-sdk/templates
 */

import type { ApiClient } from './client';
import type {
  Template,
  TemplateRegistry,
  PaginationParams,
} from './types';

// =============================================================================
// TemplateManager
// =============================================================================

/**
 * Manages DApp templates in the X3 Foundry registry.
 *
 * Provides comprehensive template discovery, registration, and forking
 * capabilities. Templates serve as reusable blueprints for creating
 * new DApp projects with pre-built structures and configurations.
 *
 * @example
 * ```typescript
 * const templates = new TemplateManager(apiClient);
 *
 * // List all available templates
 * const registry = await templates.listTemplates();
 *
 * // Fork a template to create a new project
 * const project = await templates.forkTemplate('template-123', 'My DApp');
 * ```
 */
export class TemplateManager {
  private readonly client: ApiClient;

  /**
   * Create a new TemplateManager instance.
   *
   * @param client - An authenticated ApiClient instance
   */
  constructor(client: ApiClient) {
    this.client = client;
  }

  /**
   * List all available templates in the registry.
   *
   * Returns a paginated registry of templates with filtering,
   * sorting, and category metadata.
   *
   * @param params - Optional pagination parameters
   * @returns A paginated template registry
   */
  async listTemplates(params?: PaginationParams): Promise<TemplateRegistry> {
    return this.client.get<TemplateRegistry>('/templates', {
      page: params?.page ?? 0,
      pageSize: params?.pageSize ?? 20,
    });
  }

  /**
   * Get a specific template by its identifier.
   *
   * Returns full template details including pricing tiers,
   * dependencies, supported chains, and metadata.
   *
   * @param templateId - The unique identifier of the template
   * @returns The full template details
   */
  async getTemplate(templateId: string): Promise<Template> {
    return this.client.get<Template>(`/templates/${templateId}`);
  }

  /**
   * List templates filtered by category.
   *
   * Returns templates that match the specified category tag,
   * with optional pagination.
   *
   * @param category - The category to filter by (e.g., "defi", "nft", "gaming")
   * @param params - Optional pagination parameters
   * @returns A paginated template registry filtered by category
   */
  async listByCategory(
    category: string,
    params?: PaginationParams
  ): Promise<TemplateRegistry> {
    return this.client.get<TemplateRegistry>('/templates', {
      category,
      page: params?.page ?? 0,
      pageSize: params?.pageSize ?? 20,
    });
  }

  /**
   * List templates that support a specific blockchain.
   *
   * Returns templates compatible with the specified chain ID,
   * with optional pagination.
   *
   * @param chainId - The blockchain chain ID to filter by
   * @param params - Optional pagination parameters
   * @returns A paginated template registry filtered by chain
   */
  async listByChain(
    chainId: number,
    params?: PaginationParams
  ): Promise<TemplateRegistry> {
    return this.client.get<TemplateRegistry>('/templates', {
      chainId,
      page: params?.page ?? 0,
      pageSize: params?.pageSize ?? 20,
    });
  }

  /**
   * Register a new template in the Foundry registry.
   *
   * Publishes a new DApp template that other creators can discover,
   * fork, and build upon. Requires the template source to be
   * uploaded to IPFS first.
   *
   * @param template - The template data to register
   * @returns The registered template with assigned ID
   */
  async registerTemplate(
    template: Omit<
      Template,
      'templateId' | 'forkCount' | 'rating' | 'ratingCount' | 'createdAt' | 'updatedAt' | 'verified'
    >
  ): Promise<Template> {
    return this.client.post<Template>('/templates', template);
  }

  /**
   * Fork an existing template to create a new project.
   *
   * Creates a new project based on the specified template,
   * copying its structure and configuration. The new project
   * will be linked to the original template via the fork lineage.
   *
   * @param templateId - The template identifier to fork
   * @param projectName - The name for the new project
   * @param config - Optional configuration overrides for the fork
   * @returns The newly created project details
   */
  async forkTemplate(
    templateId: string,
    projectName: string,
    config?: Record<string, unknown>
  ): Promise<{
    projectId: string;
    name: string;
    templateId: string;
    forkLineage: {
      parentId: string;
      depth: number;
      rootId: string;
    };
    createdAt: string;
  }> {
    return this.client.post<{
      projectId: string;
      name: string;
      templateId: string;
      forkLineage: {
        parentId: string;
        depth: number;
        rootId: string;
      };
      createdAt: string;
    }>('/templates/fork', {
      templateId,
      projectName,
      config,
    });
  }
}
