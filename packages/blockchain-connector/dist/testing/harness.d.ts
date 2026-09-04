/**
 * Test Harness — Benchmark and validation test profiles for blockchain connectors.
 *
 * Includes: latency, throughput, reorg simulation, edge-case, validator health,
 * GPU benchmark, pool performance, and custom user-defined tests.
 */
import type { TestProfile, TestRun } from "../types";
import { ConnectorManager } from "../connector/manager";
export declare const TEST_PROFILES: TestProfile[];
export declare class TestRunner {
    private manager;
    private runs;
    constructor(manager: ConnectorManager);
    /**
     * Get available test profiles for a given chain.
     */
    getProfiles(chainId?: string): TestProfile[];
    /**
     * Run a test profile against a connector.
     */
    runTest(connectorId: string, profileId: string): Promise<TestRun>;
    /**
     * Get a previous test run.
     */
    getRun(id: string): TestRun | undefined;
    /**
     * List all test runs.
     */
    listRuns(): TestRun[];
    private executeTest;
    private computeSummary;
}
//# sourceMappingURL=harness.d.ts.map