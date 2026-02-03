import { NextResponse } from 'next/server'

// Force dynamic rendering - no caching
export const dynamic = 'force-dynamic'
export const revalidate = 0

export async function GET() {
  const dbUrl = process.env.DATABASE_URL

  if (!dbUrl) {
    return NextResponse.json({
      players_tracked: 0,
      transactions_count: 0,
      wins_tracked: 0,
      rounds_tracked: 0,
    })
  }

  try {
    const { Pool } = await import('pg')
    const pool = new Pool({ connectionString: dbUrl, ssl: { rejectUnauthorized: false } })
    
    // Get counts from various tables
    const [players, transactions, wins, rounds] = await Promise.all([
      pool.query('SELECT COUNT(*) FROM player_performance'),
      pool.query('SELECT COUNT(*) FROM transactions'),
      pool.query('SELECT COUNT(*) FROM win_records'),
      pool.query('SELECT COUNT(*) FROM rounds'),
    ])
    
    await pool.end()

    return NextResponse.json({
      players_tracked: parseInt(players.rows[0]?.count || '0'),
      transactions_count: parseInt(transactions.rows[0]?.count || '0'),
      wins_tracked: parseInt(wins.rows[0]?.count || '0'),
      rounds_tracked: parseInt(rounds.rows[0]?.count || '0'),
    })
  } catch (error) {
    console.error('Database error:', error)
    return NextResponse.json({
      players_tracked: 0,
      transactions_count: 0,
      wins_tracked: 0,
      rounds_tracked: 0,
      error: String(error),
    })
  }
}
