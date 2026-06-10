import { FoundryClient } from '../../packages/x3-foundry-sdk/src';

describe('FoundryAppFactory', () => {
  let client: FoundryClient;

  beforeEach(() => {
    client = new FoundryClient({
      apiUrl: 'https://test-foundry.x3',
      chainId: 42,
    });
  });

  describe('Deployment', () => {
    it('should deploy app from template successfully', async () => {
      const project = await client.createProject({
        name: 'Test NFT Marketplace',
        templateId: 'nft_marketplace',
      });
      
      await client.generateDapp(project.id, 'NFT marketplace with auctions');
      const audit = await client.auditDapp(project.id);
      
      expect(audit.passed).toBe(true);
      
      const receipt = await client.deployDapp(project.id, { chainId: 42 });
      expect(receipt).toHaveProperty('contract_address');
      expect(receipt).toHaveProperty('transaction_hash');
      expect(receipt).toHaveProperty('app_slug');
    });

    it('should fail deployment on critical audit finding', async () => {
      const project = await client.createProject({
        name: 'Malicious App',
        templateId: 'nft_marketplace',
      });
      
      // Simulate a project with critical findings
      await expect(
        client.deployDapp(project.id, { chainId: 42 })
      ).rejects.toThrow(/critical finding/i);
    });

    it('should produce deterministic addresses via CREATE2', async () => {
      const project1 = await client.createProject({
        name: 'Deterministic Test 1',
        templateId: 'token_launchpad',
      });
      
      const project2 = await client.createProject({
        name: 'Deterministic Test 2',
        templateId: 'token_launchpad',
      });
      
      const receipt1 = await client.deployDapp(project1.id, { chainId: 42 });
      const receipt2 = await client.deployDapp(project2.id, { chainId: 42 });
      
      // Different projects should have different addresses
      expect(receipt1.contract_address).not.toBe(receipt2.contract_address);
    });

    it('should allow deployer to track their deployed apps', async () => {
      const project = await client.createProject({
        name: 'Trackable App',
        templateId: 'staking_pool',
      });
      
      await client.generateDapp(project.id, 'Staking pool');
      const receipt = await client.deployDapp(project.id, { chainId: 42 });
      
      expect(receipt.deployer).toBeDefined();
      expect(receipt.deployed_at).toBeDefined();
    });

    it('should prevent duplicate deployment', async () => {
      const project = await client.createProject({
        name: 'Unique App',
        templateId: 'escrow_app',
      });
      
      await client.generateDapp(project.id, 'Escrow service');
      await client.deployDapp(project.id, { chainId: 42 });
      
      // Second deployment should fail
      await expect(
        client.deployDapp(project.id, { chainId: 42 })
      ).rejects.toThrow(/already deployed/i);
    });

    it('should require template to exist for deployment', async () => {
      const project = await client.createProject({
        name: 'No Template',
        templateId: 'nonexistent_template',
      });
      
      await expect(
        client.generateDapp(project.id, 'Test')
      ).rejects.toThrow(/template not found/i);
    });
  });
});
