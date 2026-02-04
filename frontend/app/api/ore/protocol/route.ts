import { NextResponse } from 'next/server'

// Force dynamic rendering
export const dynamic = 'force-dynamic'
export const revalidate = 0

export async function GET() {
  // Return mock protocol stats for now
  return NextResponse.json({
    treasury_balance_lamports: 0,
    treasury_balance_sol: 0,
    motherlode_lamports: 0,
    motherlode_sol: 0,
    total_staked_ore: 0,
    total_refined_ore: 0,
    total_unclaimed_ore: 0,
    ore_price_usd: null,
    sol_price_usd: null,
  })
}
