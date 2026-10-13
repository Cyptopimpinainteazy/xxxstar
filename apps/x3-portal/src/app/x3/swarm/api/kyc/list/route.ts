import { NextResponse } from 'next/server'
import { kycDB } from '@/lib/db'

export async function GET() {
  try {
    const kyc = await kycDB.getAll()
    return NextResponse.json({ kyc })
  } catch (error) {
    console.error('Error fetching KYC entries:', error)
    return NextResponse.json({ error: 'Failed to fetch KYC entries' }, { status: 500 })
  }
}