import { NextResponse } from 'next/server'
import { pendingActionDB, eventDB } from '@/lib/db'

export async function POST(req: Request) {
  try {
    const body = await req.json()
    const { actionId } = body

    const pendingActions = await pendingActionDB.getAll()
    const pendingAction = pendingActions.find(p => p.action_id === actionId)
    if (!pendingAction) return NextResponse.json({ error: 'not_found' }, { status: 404 })

    await pendingActionDB.delete(pendingAction.id)

    // Emit event
    await eventDB.create('payout_rejected', {
      actionId,
      contributorId: pendingAction.contributor_id,
      wallet: pendingAction.wallet,
      amount: pendingAction.amount
    })

    return NextResponse.json({ result: 'rejected' })
  } catch (e) {
    console.error('Error rejecting action:', e)
    return NextResponse.json({ error: String(e) }, { status: 500 })
  }
}