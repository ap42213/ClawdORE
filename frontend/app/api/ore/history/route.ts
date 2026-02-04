import { NextResponse } from 'next/server'

// Force dynamic rendering
export const dynamic = 'force-dynamic'
export const revalidate = 0

export async function GET() {
  // Return empty history for now - would need to fetch historical rounds
  return NextResponse.json({
    rounds: [],
    count: 0,
  })
}
