import React, { useEffect, useState } from 'react';
import { useFundingPipeline, useDorksSearch } from '../../lib/x3/hooks';
import { Card, Progress, Button, Badge } from '@radix-ui/themes';
import { PieChart, BarChart, Legend, Tooltip, ResponsiveContainer, Pie, Bar, XAxis, YAxis, CartesianGrid } from 'recharts';

export function FundingDashboard() {
  const { analytics, getFundingAnalytics } = useFundingPipeline();
  const { getInvestorMatches } = useDorksSearch();
  const [investorMatches, setInvestorMatches] = useState<any[]>([]);

  useEffect(() => {
    getFundingAnalytics();
    getInvestorMatches(['AI', 'SaaS'], 1000000).then(setInvestorMatches);
  }, []);

  if (!analytics) return <div>Loading...</div>;

  const fundingPercent = (analytics.totalRaisedUsd / analytics.totalTargetUsd) * 100;
  const sourcesData = [
    { name: 'VC', value: analytics.fromVcUsd },
    { name: 'Angel', value: analytics.fromAngelUsd },
    { name: 'Grants', value: analytics.fromGrantsUsd },
    { name: 'Corporate', value: analytics.fromCorporateUsd },
  ];

  const pipelineData = [
    { name: 'In Pipeline', value: analytics.investorsInPipeline },
    { name: 'Interested', value: analytics.investorsInterested },
    { name: 'Committed', value: analytics.investorsCommitted },
  ];

  return (
    <div className="space-y-6">
      {/* Key Metrics */}
      <div className="grid grid-cols-4 gap-4">
        <Card>
          <h4 className="text-sm text-gray-600">Total Target</h4>
          <p className="text-2xl font-bold">${(analytics.totalTargetUsd / 1000000).toFixed(1)}M</p>
        </Card>

        <Card>
          <h4 className="text-sm text-gray-600">Total Raised</h4>
          <p className="text-2xl font-bold">${(analytics.totalRaisedUsd / 1000000).toFixed(1)}M</p>
        </Card>

        <Card>
          <h4 className="text-sm text-gray-600">Funding Gap</h4>
          <p className="text-2xl font-bold">${(analytics.fundingGapUsd / 1000000).toFixed(1)}M</p>
        </Card>

        <Card>
          <h4 className="text-sm text-gray-600">Success Probability</h4>
          <p className="text-2xl font-bold">{analytics.successProbabilityPercentage.toFixed(0)}%</p>
        </Card>
      </div>

      {/* Funding Progress */}
      <Card>
        <h3 className="text-lg font-semibold mb-4">Funding Progress</h3>
        <Progress 
          value={fundingPercent}
          max={100}
          className="mb-4"
        />
        <div className="grid grid-cols-3 gap-4">
          <div>
            <p className="text-sm text-gray-600">Progress</p>
            <p className="text-xl font-bold">{fundingPercent.toFixed(1)}%</p>
          </div>
          <div>
            <p className="text-sm text-gray-600">Months to Close</p>
            <p className="text-xl font-bold">{analytics.monthsToClose}</p>
          </div>
          <div>
            <p className="text-sm text-gray-600">Estimated Close</p>
            <p className="text-xl font-bold">{new Date(analytics.estimatedCloseDate).toLocaleDateString()}</p>
          </div>
        </div>
      </Card>

      {/* Sources Breakdown */}
      <Card>
        <h3 className="text-lg font-semibold mb-4">Funding Sources</h3>
        <ResponsiveContainer width="100%" height={300}>
          <PieChart>
            <Pie 
              data={sourcesData}
              dataKey="value"
              nameKey="name"
              label
            />
            <Tooltip formatter={(value) => `$${(value / 1000000).toFixed(1)}M`} />
            <Legend />
          </PieChart>
        </ResponsiveContainer>
      </Card>

      {/* Top Investor Matches */}
      <Card>
        <h3 className="text-lg font-semibold mb-4">Top Investor Matches</h3>
        {investorMatches.slice(0, 5).map(match => (
          <div key={match.investorId} className="p-3 border rounded mb-2">
            <div className="flex justify-between items-center mb-2">
              <h4 className="font-semibold">{match.investorName}</h4>
              <Badge color={match.matchScore > 80 ? 'green' : 'yellow'}>
                {match.matchScore}/100
              </Badge>
            </div>
            <Progress value={match.matchScore} max={100} />
            <div className="grid grid-cols-4 gap-2 mt-2 text-xs">
              <div>
                <p className="text-gray-600">Sector</p>
                <p className="font-semibold">{(match.sectorAlignment * 100).toFixed(0)}%</p>
              </div>
              <div>
                <p className="text-gray-600">Stage</p>
                <p className="font-semibold">{(match.stageAlignment * 100).toFixed(0)}%</p>
              </div>
              <div>
                <p className="text-gray-600">Ticket</p>
                <p className="font-semibold">{(match.ticketSizeAlignment * 100).toFixed(0)}%</p>
              </div>
              <div>
                <p className="text-gray-600">Contact Prob</p>
                <p className="font-semibold">{(match.contactProbability * 100).toFixed(0)}%</p>
              </div>
            </div>
            <Button size="small" className="w-full mt-2" variant="outline">
              View Profile
            </Button>
          </div>
        ))}
      </Card>

      {/* Investor Pipeline Summary */}
      <Card>
        <h3 className="text-lg font-semibold mb-4">Investor Pipeline</h3>
        <div className="grid grid-cols-3 gap-4">
          {pipelineData.map(item => (
            <div key={item.name} className="text-center">
              <p className="text-sm text-gray-600 mb-2">{item.name}</p>
              <p className="text-3xl font-bold">{item.value}</p>
            </div>
          ))}
        </div>
      </Card>
    </div>
  );
}