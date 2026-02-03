import { NextResponse } from 'next/server'

// Force dynamic rendering - no caching
export const dynamic = 'force-dynamic'
export const revalidate = 0

export async function GET() {
  const dbUrl = process.env.DATABASE_URL

  if (!dbUrl) {
    return NextResponse.json({
      wins: 0,
      rounds: 0,
      ore_earned: 0,
      players_tracked: 0,
      transactions_count: 0,
    })
  }

  try {
    const { Pool } = await import('pg')
    const pool = new Pool({ connectionString: dbUrl, ssl: { rejectUnauthorized: false } })
    
    // Get our wins (where we deployed on winning square)
    // Get total rounds tracked
    // Get ORE earned from wins
    const [wins, rounds, players, transactions] = await Promise.all([
      pool.query(`
        SELECT COUNT(*) as wins, COALESCE(SUM(reward_amount), 0) as ore_earned 
        FROM win_records 
        WHERE our_win = true OR reward_amount > 0
      `),
      pool.query('SELECT COUNT(*) FROM rounds'),
      pool.query('SELECT COUNT(*) FROM player_performance'),
      pool.query('SELECT COUNT(*) FROM transactions'),
    ])
    
    await pool.end()

    const winsCount = parseInt(wins.rows[0]?.wins || '0')
    const oreEarned = parseFloat(wins.rows[0]?.ore_earned || '0')
    const roundsCount = parseInt(rounds.rows[0]?.count || '0')

    return NextResponse.json({
      wins: winsCount,
      rounds: roundsCount,
      ore_earned: oreEarned,
      win_rate: roundsCount > 0 ? (winsCount / roundsCount * 100).toFixed(1) : '0',
      players_tracked: parseInt(players.rows[0]?.count || '0'),
      transactions_count: parseInt(transactions.rows[0]?.count || '0'),
    })
  } catch (error) {
    console.error('Database error:', error)
    return NextResponse.json({
      wins: 0,
      rounds: 0,
      ore_earned: 0,
      players_tracked: 0,
      transactions_count: 0,
      error: String(error),
    })
  }
}
