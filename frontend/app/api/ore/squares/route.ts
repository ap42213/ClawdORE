import { NextResponse } from 'next/server'

// Force dynamic rendering
export const dynamic = 'force-dynamic'
export const revalidate = 0

export async function GET() {
  // Generate neutral analysis for all 25 squares
  const squares = Array.from({ length: 25 }, (_, i) => ({
    square_num: i + 1,
    total_deployed_sol: 0,
    times_won: 0,
    win_rate: 0.04, // Expected 4% for random
    average_deployment: 0,
    expected_value: 0,
    recommendation: '➖ NEUTRAL',
  }))

  return NextResponse.json({
    squares,
  })
}
