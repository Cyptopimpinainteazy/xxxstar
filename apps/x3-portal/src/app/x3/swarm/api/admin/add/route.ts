import { NextResponse } from 'next/server'
import { adminDB } from '@/lib/db'
import { verifyToken } from '@/lib/auth'

export async function POST(req: Request) {
  try {
    const auth = req.headers.get('authorization') || ''
    if (!auth.startsWith('Bearer ')) return NextResponse.json({ error: 'unauth' }, { status: 401 })
    const token = auth.split(' ')[1]
    const payload = verifyToken(token as string)
    if (!payload) return NextResponse.json({ error: 'invalid_token' }, { status: 401 })

    const body = await req.json()
    const { admin } = body
    if (!admin) return NextResponse.json({ error: 'missing' }, { status: 400 })

    await adminDB.add(admin)
    return NextResponse.json({ result: 'ok' })
  } catch (e) {
    console.error('Error adding admin:', e)
    return NextResponse.json({ error: String(e) }, { status: 500 })
  }
}