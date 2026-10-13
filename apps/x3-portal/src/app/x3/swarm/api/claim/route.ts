import { NextResponse } from 'next/server'
import { consentDB, claimDB, pendingActionDB, allocationDB } from '@/lib/db'

export async function POST(req: Request) {
  try {
    const body = await req.json()
    const { contributorId, wallet } = body

    // Get allocation from DB
    const allocations = await allocationDB.getAll()
    const alloc = allocations.find(a => a.contributor_id === contributorId)?.amount || 0
    if (alloc === 0) return NextResponse.json({ error: 'no_allocation' }, { status: 400 })

    // Check consent
    const consents = await consentDB.getAll()
    const hasConsent = consents.some(c => c.contributor_id === contributorId && c.wallet === wallet)
    if (!hasConsent) {
      return NextResponse.json({ error: 'consent_required' }, { status: 403 })
    }

    // Governance threshold gating
    const threshold = Number(process.env.CLAIM_GOV_THRESHOLD || 100)
    if (alloc > threshold) {
      // Queue governance action
      const actionId = `claim-${contributorId}-${Date.now()}`
      await pendingActionDB.create(actionId, contributorId, wallet, alloc)
      return NextResponse.json({ result: 'pending_approval', actionId })
    }

    // Accept small claims immediately
    await claimDB.create(contributorId, wallet, alloc)
    return NextResponse.json({ result: 'queued', amount: alloc })
  } catch (e) {
    console.error('Error processing claim:', e)
    return NextResponse.json({ error: String(e) }, { status: 500 })
  }
}