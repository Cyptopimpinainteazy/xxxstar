import { NextResponse } from 'next/server'
import { pendingActionDB, adminDB, kycDB, eventDB } from '@/lib/db'
import { ethers } from 'ethers'
import { spawn } from 'child_process'

export async function POST(req: Request) {
  try {
    const body = await req.json()
    const { actionId, signature } = body

    // Get pending action from DB
    const pendingActions = await pendingActionDB.getAll()
    const pendingAction = pendingActions.find(p => p.action_id === actionId)
    if (!pendingAction) return NextResponse.json({ error: 'not_found' }, { status: 404 })

    // verify signature and admin membership
    if (!signature) return NextResponse.json({ error: 'missing_signature' }, { status: 400 })
    let signer = null
    try {
      signer = ethers.verifyMessage(actionId, signature)
    } catch (e) {
      return NextResponse.json({ error: 'invalid_signature' }, { status: 400 })
    }
    const isAdmin = await adminDB.exists(signer)
    if (!isAdmin) return NextResponse.json({ error: 'not_admin' }, { status: 403 })

    // register approval
    const approvals = pendingAction.approvals || []
    if (!approvals.includes(signer)) {
      approvals.push(signer)
      await pendingActionDB.addApproval(pendingAction.id, signer)
    }

    // KYC threshold gating
    const kycThreshold = Number(process.env.KYC_THRESHOLD || 100)
    if (Number(pendingAction.amount) >= kycThreshold) {
      const kycEntries = await kycDB.getAll()
      const hasKyc = kycEntries.some(k =>
        k.contributor_id === pendingAction.contributor_id &&
        k.wallet === pendingAction.wallet &&
        k.verified === true
      )
      if (!hasKyc) return NextResponse.json({ error: 'kyc_required' }, { status: 403 })
    }

    // simple threshold: if approvals >= GOV_APPROVAL_THRESHOLD finalize
    const threshold = Number(process.env.GOV_APPROVAL_THRESHOLD || 1)
    if (approvals.length >= threshold) {
      // Move to finalized payouts
      await pendingActionDB.delete(pendingAction.id)

      // Emit event
      await eventDB.create('payout_finalized', {
        actionId,
        contributorId: pendingAction.contributor_id,
        wallet: pendingAction.wallet,
        amount: pendingAction.amount,
        meta: pendingAction.meta
      })

      // optional auto-fund on finalize
      const autoFund = process.env.AUTO_FUND_ON_FINALIZE === '1'
      if (autoFund) {
        try {
          // call CLI helper to process finalized payouts (non-blocking)
          spawn('python', ['tools/process_finalized_payouts.py'], { stdio: 'inherit' })
          // Don't wait for completion
        } catch (e) {
          console.error('Failed to spawn process_finalized_payouts helper:', e)
        }
      }

      return NextResponse.json({ result: 'finalized' })
    }
    return NextResponse.json({ result: 'approved' })
  } catch (e) {
    console.error('Error approving action:', e)
    return NextResponse.json({ error: String(e) }, { status: 500 })
  }
}