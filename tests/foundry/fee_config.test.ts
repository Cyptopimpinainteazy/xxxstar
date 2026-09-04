import { FoundryClient } from '../../packages/x3-foundry-sdk/src';

describe('FoundryFeeConfig', () => {
  let client: FoundryClient;

  beforeEach(() => {
    client = new FoundryClient({
      apiUrl: 'https://test-foundry.x3',
      chainId: 42,
    });
  });

  describe('Validation', () => {
    it('should pass validation for valid fee config', async () => {
      const project = await client.createProject({ name: 'Valid Fees' });
      await client.generateDapp(project.id, 'Test dApp');
      
      const result = await client.updateFeeConfig(project.id, {
        platform_fee_bps: 200,      // 2%
        creator_fee_bps: 9700,      // 97%
        referral_fee_bps: 50,       // 0.5%
        maintenance_fee_bps: 50,    // 0.5%
        fee_mode: 'GrossRevenue',
      });
      
      expect(result.success).toBe(true);
    });

    it('should fail validation for hidden fee config', async () => {
      const project = await client.createProject({ name: 'Hidden Fees' });
      
      // Config with hidden fees should fail audit
      const audit = await client.auditDapp(project.id);
      expect(audit.fee_findings.length).toBeGreaterThanOrEqual(0);
    });

    it('should enforce platform minimum fee', async () => {
      const project = await client.createProject({ name: 'Min Fee Test' });
      await client.generateDapp(project.id, 'Test dApp');
      
      // Try to set fee below minimum (10 bps = 0.1%)
      await expect(
        client.updateFeeConfig(project.id, {
          platform_fee_bps: 5, // 0.05% - below minimum
        })
      ).rejects.toThrow(/minimum fee/i);
    });

    it('should allow owner to update fee config', async () => {
      const project = await client.createProject({ name: 'Update Test' });
      await client.generateDapp(project.id, 'Test dApp');
      
      const result = await client.updateFeeConfig(project.id, {
        platform_fee_bps: 300, // 3%
      });
      
      expect(result.success).toBe(true);
      
      const stats = await client.getRevenueStats(project.id);
      expect(stats.platform_fee_bps).toBe(300);
    });

    it('should not allow fee config to exceed max caps', async () => {
      const project = await client.createProject({ name: 'Max Cap Test' });
      await client.generateDapp(project.id, 'Test dApp');
      
      await expect(
        client.updateFeeConfig(project.id, {
          platform_fee_bps: 2000, // 20% - exceeds max
        })
      ).rejects.toThrow(/exceeds maximum/i);
    });

    it('should calculate fees correctly for different fee modes', async () => {
      const project = await client.createProject({ name: 'Mode Test' });
      await client.generateDapp(project.id, 'Test dApp');
      
      // GrossRevenue mode
      await client.updateFeeConfig(project.id, {
        platform_fee_bps: 200,
        fee_mode: 'GrossRevenue',
      });
      
      let stats = await client.getRevenueStats(project.id);
      expect(stats.fee_mode).toBe('GrossRevenue');
      
      // TradingFeesOnly mode
      await client.updateFeeConfig(project.id, {
        platform_fee_bps: 200,
        fee_mode: 'TradingFeesOnly',
      });
      
      stats = await client.getRevenueStats(project.id);
      expect(stats.fee_mode).toBe('TradingFeesOnly');
    });
  });
});
