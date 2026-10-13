import { NextResponse } from 'next/server'
import { allocationDB } from '@/lib/db'

export async function GET() {
  try {
    const allocations = await allocationDB.getAll()
    const allocation = allocations.reduce((acc, a) => {
      acc[a.contributor_id] = a.amount
      return acc
    }, {} as Record<string, number>)

    // For now, return basic structure
    return NextResponse.json({
      contributors: [], // TODO: add contributor details if needed
      allocation
    })
  } catch (error) {
    console.error('Error fetching contributors:', error)
    // Fallback sample
    const sample = {
      contributors: [
        { id: 'alice_0', hours: 12.5, gflops: 512 },
        { id: 'bob_1', hours: 8.1, gflops: 256 },
        { id: 'carol_2', hours: 5.2, gflops: 128 },
      ],
      allocation: { alice_0: 600.0, bob_1: 300.0, carol_2: 100.0 }
    }
    return NextResponse.json(sample)
  }
}