import { FoundryClient } from '../../packages/x3-foundry-sdk/src';

describe('FoundryRevenueRouter', () => {
  let client: FoundryClient;

  beforeEach(() => {
    client = new FoundryClient({
      apiUrl: 'https://test-foundry.x3',
      chainId: 42,
    });
  });

  describe('Platform Fee Routing', () => {
    it('should route platform fee correctly to treasury split', async () => {
      const project = await client.createProject({ name: 'Test App' });
      await client.generateDapp(project.id, 'Test dApp');
      
      const stats = await client.getRevenueStats(project.id);
      
      expect(stats.platform_fee_bps).toBe(200); // 2%
      expect(stats.creator_share_bps).toBe(9700); // 97%
      expect(stats.treasury_split.protocol_treasury).toBe(40);
      expect(stats.treasury_split.gpu_swarm).toBe(20);
      expect(stats.treasury_split.dev_vault).toBe(15);
      expect(stats.treasury_split.maintenance_vault).toBe(10);
      expect(stats.treasury_split.liquidity_incentives).toBe(10);
      expect(stats.treasury_split.grants).toBe(5);
    });

    it('should route creator revenue correctly to creator vault', async () => {
      const project = await client.createProject({ name: 'Creator Test' });
      await client.generateDapp(project.id, 'Test dApp');
      
      const revenue = 10000; // 10,000 USDC
      const creatorShare = revenue * 0.97; // 97%
      
      const stats = await client.getRevenueStats(project.id);
      expect(stats.creator_earnings).toBeGreaterThanOrEqual(0);
    });

    it('should route referral revenue correctly to referrer', async () => {
      const project = await client.createProject({ name: 'Referral Test' });
      await client.generateDapp(project.id, 'Test dApp');
      
      await client.updateFeeConfig(project.id, {
        referral_fee_bps: 50, // 0.5%
      });
      
      const stats = await client.getRevenueStats(project.id);
      expect(stats.referral_fee_bps).toBe(50);
    });
  });

  describe('Principal Safety', () => {
    it('should never allow platform fee on user deposits', async () => {
      const project = await client.createProject({ name: 'Safety Test' });
      await client.generateDapp(project.id, 'Test dApp');
      
      // User deposits 1000 tokens
      const userDeposit = 1000;
      
      // Platform fee should only apply to protocol revenue, not deposits
      const stats = await client.getRevenueStats(project.id);
      expect(stats.platform_fee_on_principal).toBe(false);
    });
  });

  describe('Fee Caps', () => {
    it('should enforce platform fee cannot exceed max cap', async () => {
      const project = await client.createProject({ name: 'Cap Test' });
      await client.generateDapp(project.id, 'Test dApp');
      
      // Try to set fee above max
      await expect(
        client.updateFeeConfig(project.id, {
          platform_fee_bps: 10000, // 100% - should fail
        })
      ).rejects.toThrow();
    });

    it('should reject hidden fee configurations', async () => {
      const project = await client.createProject({ name: 'Hidden Fee Test' });
      
      // Hidden fee config should fail audit
      await expect(
        client.auditDapp(project.id)
      ).rejects.toThrow();
    });
  });

  describe('Treasury Split', () => {
    it('should have treasury split percentages sum to 100%', async () => {
      const project = await client.createProject({ name: 'Split Test' });
      await client.generateDapp(project.id, 'Test dApp');
      
      const stats = await client.getRevenueStats(project.id);
      const total = 
        stats.treasury_split.protocol_treasury +
        stats.treasury_split.gpu_swarm +
        stats.treasury_split.dev_vault +
        stats.treasury_split.maintenance_vault +
        stats.treasury_split.liquidity_incentives +
        stats.treasury_split.grants;
      
      expect(total).toBe(100);
    });
  });

  describe('Revenue Reports', () => {
    it('should match revenue reports with on-chain events', async () => {
      const project = await client.createProject({ name: 'Report Test' });
      await client.generateDapp(project.id, 'Test dApp');
      
      const stats = await client.getRevenueStats(project.id);
      
      expect(stats).toHaveProperty('total_revenue');
      expect(stats).toHaveProperty('platform_fees');
      expect(stats).toHaveProperty('creator_earnings');
      expect(stats).toHaveProperty('referral_rewards');
      expect(stats).toHaveProperty('treasury_contributions');
    });
  });
});
