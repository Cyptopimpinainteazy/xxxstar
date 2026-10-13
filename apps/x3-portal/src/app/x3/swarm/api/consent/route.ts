import { NextResponse } from 'next/server'
import { consentDB } from '@/lib/db'

export async function POST(req: Request) {
  try {
    const body = await req.json()
    const { contributorId, wallet, kyc } = body
    if (!contributorId || !wallet) return NextResponse.json({ error: 'missing' }, { status: 400 })

    await consentDB.create(contributorId, wallet, !!kyc)
    return NextResponse.json({ result: 'ok' })
  } catch (e) {
    console.error('Error creating consent:', e)
    return NextResponse.json({ error: String(e) }, { status: 500 })
  }
}