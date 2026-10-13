import { NextResponse } from 'next/server'
import { kycDB } from '@/lib/db'
import crypto from 'crypto'

export async function POST(req: Request) {
  try {
    const body = await req.json()
    const { contributorId, wallet, provider, ref, verified, signature } = body

    // TODO: Verify signature using provider secret (for now, assume valid)
    // In production, store provider secrets securely (Vault/env)
    const secret = process.env[`KYC_PROVIDER_${provider.toUpperCase()}_SECRET`]
    if (!secret) return NextResponse.json({ error: 'unknown_provider' }, { status: 400 })

    const payload = `${contributorId}|${wallet}|${provider}|${ref}|${verified}`
    const mac = crypto.createHmac('sha256', secret).update(payload).digest('hex')
    if (mac !== signature) return NextResponse.json({ error: 'invalid_signature' }, { status: 403 })

    // Store KYC entry
    await kycDB.create(contributorId, wallet, provider, ref)
    if (verified) {
      const entries = await kycDB.getAll()
      const latest = entries.find(k => k.contributor_id === contributorId && k.wallet === wallet && k.provider === provider)
      if (latest) await kycDB.verify(latest.id, provider)
    }

    return NextResponse.json({ result: 'ok' })
  } catch (e) {
    console.error('Error processing KYC notification:', e)
    return NextResponse.json({ error: String(e) }, { status: 500 })
  }
}