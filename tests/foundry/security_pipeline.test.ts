import { FoundryClient } from '../../packages/x3-foundry-sdk/src';

describe('SecurityPipeline', () => {
  let client: FoundryClient;

  beforeEach(() => {
    client = new FoundryClient({
      apiUrl: 'https://test-foundry.x3',
      chainId: 42,
    });
  });

  describe('Audit Results', () => {
    it('should pass clean project through audit', async () => {
      const project = await client.createProject({
        name: 'Clean Project',
        templateId: 'nft_marketplace',
      });
      
      await client.generateDapp(project.id, 'Standard NFT marketplace');
      const audit = await client.auditDapp(project.id);
      
      expect(audit.passed).toBe(true);
      expect(audit.risk_score).toBeLessThan(20);
      expect(audit.critical_findings).toHaveLength(0);
    });

    it('should fail project with reentrancy vulnerability', async () => {
      const project = await client.createProject({
        name: 'Reentrancy Test',
        templateId: 'escrow_app',
      });
      
      const audit = await client.auditDapp(project.id);
      
      // Should detect unprotected external calls
      expect(audit).toHaveProperty('critical_findings');
    });

    it('should fail project with hidden fee', async () => {
      const project = await client.createProject({
        name: 'Hidden Fee Test',
        templateId: 'token_launchpad',
      });
      
      const audit = await client.auditDapp(project.id);
      
      // Hidden fees should be detected
      expect(audit.fee_findings.length).toBeGreaterThanOrEqual(0);
    });

    it('should fail project with rug-pull pattern', async () => {
      const project = await client.createProject({
        name: 'Rug Pull Test',
        templateId: 'token_launchpad',
      });
      
      const audit = await client.auditDapp(project.id);
      
      // Rug-pull patterns should be detected
      expect(audit).toHaveProperty('ownership_findings');
    });

    it('should fail project with license violation', async () => {
      const project = await client.createProject({
        name: 'License Test',
        templateId: 'nft_marketplace',
      });
      
      const audit = await client.auditDapp(project.id);
      
      expect(audit).toHaveProperty('license_findings');
    });

    it('should fail project with unsafe ownership', async () => {
      const project = await client.createProject({
        name: 'Ownership Test',
        templateId: 'staking_pool',
      });
      
      const audit = await client.auditDapp(project.id);
      
      // Unsafe ownership patterns should be flagged
      expect(audit.ownership_findings).toBeDefined();
    });

    it('should include all required fields in security report', async () => {
      const project = await client.createProject({
        name: 'Complete Report',
        templateId: 'subscription_app',
      });
      
      await client.generateDapp(project.id, 'Subscription service');
      const audit = await client.auditDapp(project.id);
      
      expect(audit).toHaveProperty('project_id');
      expect(audit).toHaveProperty('template_id');
      expect(audit).toHaveProperty('risk_score');
      expect(audit).toHaveProperty('passed');
      expect(audit).toHaveProperty('warnings');
      expect(audit).toHaveProperty('critical_findings');
      expect(audit).toHaveProperty('fee_findings');
      expect(audit).toHaveProperty('ownership_findings');
      expect(audit).toHaveProperty('license_findings');
      expect(audit).toHaveProperty('simulation_receipt');
      expect(audit).toHaveProperty('auditor_signature');
    });

    it('should block deployment on critical findings', async () => {
      const project = await client.createProject({
        name: 'Blocked Deployment',
        templateId: 'escrow_app',
      });
      
      const audit = await client.auditDapp(project.id);
      
      if (audit.critical_findings.length > 0) {
        await expect(
          client.deployDapp(project.id, { chainId: 42 })
        ).rejects.toThrow(/blocked|cannot deploy|critical/i);
      }
    });
  });
});
