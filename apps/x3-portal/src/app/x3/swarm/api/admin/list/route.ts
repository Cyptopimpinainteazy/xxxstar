import { NextResponse } from 'next/server'
import { adminDB } from '@/lib/db'

export async function GET() {
  try {
    const admins = await adminDB.getAll()
    return NextResponse.json({ admins })
  } catch (error) {
    console.error('Error fetching admins:', error)
    return NextResponse.json({ error: 'Failed to fetch admins' }, { status: 500 })
  }
}