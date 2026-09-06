const formatCurrency = (amount, currency) => new Intl.NumberFormat('en-US', {
  style: 'currency', currency, maximumFractionDigits: 0,
}).format(amount)

export function toDisplayModel(portal) {
  const { funding, completion } = portal
  const allFundingIsUnrecorded = ['pledged', 'received', 'allocated', 'spent'].every((field) => funding[field] === 0)
  return {
    funnel: [
      { id: 'requested', label: 'Requested', value: funding.requested, formatted: formatCurrency(funding.requested, funding.currency), detail: 'Declared funding request' },
      { id: 'pledged', label: 'Pledged', value: funding.pledged, formatted: formatCurrency(funding.pledged, funding.currency), detail: 'Documented commitments only' },
      { id: 'received', label: 'Received', value: funding.received, formatted: formatCurrency(funding.received, funding.currency), detail: 'Receipts or transaction records required' },
      { id: 'allocated', label: 'Allocated', value: funding.allocated, formatted: formatCurrency(funding.allocated, funding.currency), detail: 'Assigned to a work package' },
      { id: 'spent', label: 'Spent with proof', value: funding.spent, formatted: formatCurrency(funding.spent, funding.currency), detail: 'Receipt or transaction reference required' },
    ],
    fundingDisclosure: allFundingIsUnrecorded
      ? 'No funding, allocation, spending, or payment receipts are recorded in this portal yet.'
      : 'Amounts are derived from the published treasury ledger. Spending records require linked proof references.',
    completion: {
      readinessScore: completion.readinessScore,
      completedTasks: completion.completedTasks,
      taskCount: completion.taskCount,
      releaseDecision: completion.releaseDecision,
      openFindings: Object.entries(completion.openFindings),
      checks: completion.checks,
    },
  }
}
