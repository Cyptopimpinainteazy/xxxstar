import { NextResponse } from 'next/server'
import { pendingActionDB } from '@/lib/db'

export async function GET() {
  try {
    const pending = await pendingActionDB.getAll()
    return NextResponse.json({ pending })
  } catch (error) {
    console.error('Error fetching pending actions:', error)
    return NextResponse.json({ error: 'Failed to fetch pending actions' }, { status: 500 })
  }
}