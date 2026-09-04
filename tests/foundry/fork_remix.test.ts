import { FoundryClient } from '../../packages/x3-foundry-sdk/src';

describe('FoundryForkRemix', () => {
  let client: FoundryClient;

  beforeEach(() => {
    client = new FoundryClient({
      apiUrl: 'https://test-foundry.x3',
      chainId: 42,
    });
  });

  describe('Fork Operations', () => {
    it('should allow forking an existing app', async () => {
      const originalProject = await client.createProject({
        name: 'Original App',
        templateId: 'nft_marketplace',
      });
      
      await client.generateDapp(originalProject.id, 'NFT marketplace');
      await client.deployDapp(originalProject.id, { chainId: 42 });
      
      const forkedProject = await client.forkProject(originalProject.id, {
        name: 'Forked Marketplace',
      });
      
      expect(forkedProject.id).not.toBe(originalProject.id);
      expect(forkedProject.forked_from).toBe(originalProject.id);
    });

    it('should track fork lineage correctly', async () => {
      const original = await client.createProject({ name: 'Original' });
      await client.generateDapp(original.id, 'Test');
      
      const fork1 = await client.forkProject(original.id, { name: 'Fork 1' });
      const fork2 = await client.forkProject(fork1.id, { name: 'Fork 2' });
      
      const lineage1 = await client.getProjectLineage(fork1.id);
      expect(lineage1.original_app_id).toBe(original.id);
      expect(lineage1.fork_depth).toBe(1);
      
      const lineage2 = await client.getProjectLineage(fork2.id);
      expect(lineage2.original_app_id).toBe(original.id);
      expect(lineage2.fork_depth).toBe(2);
    });

    it('should pay remix royalty to original creator', async () => {
      const original = await client.createProject({ name: 'Original' });
      await client.generateDapp(original.id, 'Test');
      
      await client.updateFeeConfig(original.id, {
        remix_royalty_bps: 50, // 0.5%
      });
      
      const fork = await client.forkProject(original.id, { name: 'Fork' });
      const stats = await client.getRevenueStats(fork.id);
      
      expect(stats.remix_royalty_bps).toBe(50);
    });

    it('should still apply platform fee to forked apps', async () => {
      const original = await client.createProject({ name: 'Original' });
      await client.generateDapp(original.id, 'Test');
      
      const fork = await client.forkProject(original.id, { name: 'Fork' });
      const stats = await client.getRevenueStats(fork.id);
      
      expect(stats.platform_fee_bps).toBe(200); // 2% platform fee still applies
    });

    it('should inherit license restrictions', async () => {
      const original = await client.createProject({ name: 'Original' });
      await client.generateDapp(original.id, 'Test');
      
      const fork = await client.forkProject(original.id, { name: 'Fork' });
      const audit = await client.auditDapp(fork.id);
      
      expect(audit.license_findings).toBeDefined();
    });

    it('should track fork count', async () => {
      const original = await client.createProject({ name: 'Original' });
      await client.generateDapp(original.id, 'Test');
      
      await client.forkProject(original.id, { name: 'Fork 1' });
      await client.forkProject(original.id, { name: 'Fork 2' });
      await client.forkProject(original.id, { name: 'Fork 3' });
      
      const stats = await client.getRevenueStats(original.id);
      expect(stats.fork_count).toBeGreaterThanOrEqual(3);
    });
  });
});
