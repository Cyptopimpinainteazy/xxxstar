import { NextResponse } from 'next/server'
import { kycDB } from '@/lib/db'
import { verifyToken } from '@/lib/auth'

export async function POST(req: Request) {
  try {
    const auth = req.headers.get('authorization') || ''
    if (!auth.startsWith('Bearer ')) return NextResponse.json({ error: 'unauth' }, { status: 401 })
    const token = auth.split(' ')[1]
    const payload = verifyToken(token as string)
    if (!payload) return NextResponse.json({ error: 'invalid_token' }, { status: 401 })

    const body = await req.json()
    const { contributorId, wallet, verified } = body
    if (!contributorId || !wallet) return NextResponse.json({ error: 'missing' }, { status: 400 })

    // Find existing KYC entry or create new
    const kycEntries = await kycDB.getAll()
    const existing = kycEntries.find(k => k.contributor_id === contributorId && k.wallet === wallet)

    if (existing) {
      await kycDB.verify(existing.id, payload.admin)
    } else {
      await kycDB.create(contributorId, wallet, 'manual', payload.admin)
      if (verified) {
        const newEntry = await kycDB.getAll()
        const latest = newEntry.find(k => k.contributor_id === contributorId && k.wallet === wallet)
        if (latest) await kycDB.verify(latest.id, payload.admin)
      }
    }

    return NextResponse.json({ result: 'ok' })
  } catch (e) {
    console.error('Error verifying KYC:', e)
    return NextResponse.json({ error: String(e) }, { status: 500 })
  }
}