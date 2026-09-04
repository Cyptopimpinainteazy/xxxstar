/**
 * X3 Marketplace SDK - JavaScript/TypeScript wrapper
 * 
 * Provides unified interface to marketplace operations:
 * - Plugin discovery and installation
 * - Rating and review management
 * - Payment processing and tracking
 * - IPFS metadata handling
 */

/**
 * Plugin object returned from registry
 */
export interface Plugin {
    id: string;
    name: string;
    category: PluginCategory;
    rating: number;
    downloads: number;
    author: string;
    version: string;
    license: string;
    icon: string;
    verified: boolean;
}

/**
 * Plugin categories
 */
export enum PluginCategory {
    Authentication = "authentication",
    Analytics = "analytics",
    Wallet = "wallet",
    Trading = "trading",
    Governance = "governance",
    Staking = "staking",
    Bridge = "bridge",
    Oracle = "oracle",
    DeFi = "defi",
    NFT = "nft",
    Social = "social",
    Other = "other",
}

/**
 * User review and rating
 */
export interface Review {
    id: string;
    reviewer: string;
    rating: 1 | 2 | 3 | 4 | 5;
    title: string;
    content: string;
    helpful: number;
    unhelpful: number;
    verified: boolean;
    date: Date;
}

/**
 * Plugin rating summary
 */
export interface RatingSummary {
    average: number;
    total: number;
    distribution: number[];
    helpful: number;
    recommended: number;
}

/**
 * Payment record
 */
export interface Payment {
    id: string;
    amount: number;
    feeAmount: number;
    yourShare: number;
    date: Date;
    status: "pending" | "completed" | "failed";
}

/**
 * Marketplace API Client
 */
export class MarketplaceClient {
    private baseUrl: string;
    private apiKey: string;
    private cache: Map<string, any> = new Map();

    constructor(baseUrl: string, apiKey: string) {
        this.baseUrl = baseUrl;
        this.apiKey = apiKey;
    }

    /**
     * Search for plugins
     */
    async searchPlugins(query: string): Promise<Plugin[]> {
        const cacheKey = `search:${query}`;
        if (this.cache.has(cacheKey)) {
            return this.cache.get(cacheKey);
        }

        const response = await this.fetch(`/plugins/search`, {
            params: { q: query },
        });

        const plugins = response.data;
        this.cache.set(cacheKey, plugins);
        return plugins;
    }

    /**
     * Get plugin by ID
     */
    async getPlugin(pluginId: string): Promise<Plugin> {
        const cacheKey = `plugin:${pluginId}`;
        if (this.cache.has(cacheKey)) {
            return this.cache.get(cacheKey);
        }

        const response = await this.fetch(`/plugins/${pluginId}`);
        this.cache.set(cacheKey, response.data);
        return response.data;
    }

    /**
     * Get trending plugins
     */
    async getTrendingPlugins(limit: number = 10): Promise<Plugin[]> {
        const response = await this.fetch(`/plugins/trending`, {
            params: { limit },
        });
        return response.data;
    }

    /**
     * Get top rated plugins
     */
    async getTopRatedPlugins(limit: number = 10): Promise<Plugin[]> {
        const response = await this.fetch(`/plugins/top-rated`, {
            params: { limit },
        });
        return response.data;
    }

    /**
     * Get plugins by category
     */
    async getPluginsByCategory(
        category: PluginCategory,
        limit: number = 20
    ): Promise<Plugin[]> {
        const response = await this.fetch(`/plugins/category/${category}`, {
            params: { limit },
        });
        return response.data;
    }

    /**
     * Get plugin reviews
     */
    async getPluginReviews(
        pluginId: string,
        limit: number = 10
    ): Promise<Review[]> {
        const response = await this.fetch(`/plugins/${pluginId}/reviews`, {
            params: { limit },
        });
        return response.data;
    }

    /**
     * Get top reviews for plugin
     */
    async getTopReviews(pluginId: string, limit: number = 5): Promise<Review[]> {
        const response = await this.fetch(`/plugins/${pluginId}/reviews/top`, {
            params: { limit },
        });
        return response.data;
    }

    /**
     * Get rating summary for plugin
     */
    async getRatingSummary(pluginId: string): Promise<RatingSummary> {
        const cacheKey = `ratings:${pluginId}`;
        if (this.cache.has(cacheKey)) {
            return this.cache.get(cacheKey);
        }

        const response = await this.fetch(`/plugins/${pluginId}/ratings`);
        this.cache.set(cacheKey, response.data);
        return response.data;
    }

    /**
     * Submit review
     */
    async submitReview(
        pluginId: string,
        rating: 1 | 2 | 3 | 4 | 5,
        title: string,
        content: string
    ): Promise<Review> {
        const response = await this.fetch(`/plugins/${pluginId}/reviews`, {
            method: "POST",
            body: { rating, title, content },
        });

        // Invalidate cache
        this.cache.delete(`ratings:${pluginId}`);
        this.cache.delete(`plugin:${pluginId}`);

        return response.data;
    }

    /**
     * Mark review as helpful
     */
    async markHelpful(reviewId: string): Promise<void> {
        await this.fetch(`/reviews/${reviewId}/helpful`, {
            method: "POST",
        });
    }

    /**
     * Get publisher earnings
     */
    async getPublisherEarnings(publisherId: string): Promise<{
        total: number;
        claimed: number;
        pending: number;
    }> {
        const response = await this.fetch(`/publishers/${publisherId}/earnings`);
        return response.data;
    }

    /**
     * Get payment history
     */
    async getPaymentHistory(limit: number = 50): Promise<Payment[]> {
        const response = await this.fetch(`/payments`, {
            params: { limit },
        });
        return response.data;
    }

    /**
     * Claim earnings
     */
    async claimEarnings(): Promise<{
        amount: number;
        transactionId: string;
    }> {
        const response = await this.fetch(`/publishers/claim`, {
            method: "POST",
        });
        return response.data;
    }

    /**
     * Install plugin (record in local registry)
     */
    async installPlugin(pluginId: string): Promise<void> {
        const response = await this.fetch(`/plugins/${pluginId}/install`, {
            method: "POST",
        });

        // Record download
        this.fetch(`/plugins/${pluginId}/download`, { method: "POST" });

        return response.data;
    }

    /**
     * Get installed plugins
     */
    async getInstalledPlugins(): Promise<Plugin[]> {
        const response = await this.fetch(`/installations`);
        return response.data;
    }

    /**
     * Uninstall plugin
     */
    async uninstallPlugin(pluginId: string): Promise<void> {
        await this.fetch(`/installations/${pluginId}`, {
            method: "DELETE",
        });
    }

    /**
     * Check plugin update availability
     */
    async checkUpdates(pluginId: string): Promise<{
        available: boolean;
        version?: string;
        changelog?: string;
    }> {
        const response = await this.fetch(`/plugins/${pluginId}/updates`);
        return response.data;
    }

    /**
     * Get IPFS metadata
     */
    async getIPFSMetadata(ipfsHash: string): Promise<any> {
        const response = await this.fetch(`/ipfs/${ipfsHash}`);
        return response.data;
    }

    /**
     * Upload metadata to IPFS
     */
    async uploadMetadata(metadata: any): Promise<{
        hash: string;
        url: string;
    }> {
        const response = await this.fetch(`/ipfs/upload`, {
            method: "POST",
            body: metadata,
        });
        return response.data;
    }

    /**
     * Clear all caches
     */
    clearCache(): void {
        this.cache.clear();
    }

    /**
     * Clear specific cache key
     */
    clearCacheKey(key: string): void {
        this.cache.delete(key);
    }

    /**
     * Internal fetch wrapper
     */
    private async fetch(
        path: string,
        options: any = {}
    ): Promise<{
        data: any;
        status: number;
    }> {
        const url = new URL(path, this.baseUrl);

        if (options.params) {
            Object.entries(options.params).forEach(([key, value]) => {
                url.searchParams.append(key, String(value));
            });
        }

        const response = await fetch(url.toString(), {
            method: options.method || "GET",
            headers: {
                "Content-Type": "application/json",
                Authorization: `Bearer ${this.apiKey}`,
                ...options.headers,
            },
            body:
                options.body && options.method !== "GET"
                    ? JSON.stringify(options.body)
                    : undefined,
        });

        const data = await response.json();

        if (!response.ok) {
            throw new Error(
                `API Error: ${response.status} - ${data.message}`
            );
        }

        return { data, status: response.status };
    }
}

/**
 * Plugin installer utility
 */
export class PluginInstaller {
    private client: MarketplaceClient;
    private installed: Map<string, string> = new Map(); // pluginId -> version

    constructor(client: MarketplaceClient) {
        this.client = client;
    }

    /**
     * Install plugin with version checking
     */
    async install(pluginId: string): Promise<boolean> {
        try {
            // Check compatibility
            const plugin = await this.client.getPlugin(pluginId);
            const isCompatible = this.checkCompatibility(plugin);

            if (!isCompatible) {
                throw new Error(
                    `Plugin ${pluginId} is not compatible with your system`
                );
            }

            // Install
            await this.client.installPlugin(pluginId);
            this.installed.set(pluginId, plugin.version);

            return true;
        } catch (error) {
            console.error(`Failed to install plugin: ${error}`);
            return false;
        }
    }

    /**
     * Check for updates
     */
    async checkForUpdates(): Promise<Array<{ pluginId: string; version: string }>> {
        const updates: Array<{ pluginId: string; version: string }> = [];

        for (const [pluginId, currentVersion] of this.installed) {
            const updateInfo = await this.client.checkUpdates(pluginId);
            if (updateInfo.available && updateInfo.version !== currentVersion) {
                updates.push({ pluginId, version: updateInfo.version! });
            }
        }

        return updates;
    }

    /**
     * Update plugin
     */
    async update(pluginId: string): Promise<boolean> {
        try {
            await this.client.uninstallPlugin(pluginId);
            return await this.install(pluginId);
        } catch (error) {
            console.error(`Failed to update plugin: ${error}`);
            return false;
        }
    }

    /**
     * Get compatibility check
     */
    private checkCompatibility(plugin: Plugin): boolean {
        // In real implementation, would check:
        // - SDK version compatibility
        // - Platform compatibility
        // - Dependency satisfaction
        return true;
    }
}

// Export version
export const VERSION = "1.0.0";
