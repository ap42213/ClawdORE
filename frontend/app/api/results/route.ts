import { NextResponse } from 'next/server'

// Force dynamic rendering - no caching
export const dynamic = 'force-dynamic'
export const revalidate = 0

export interface RoundResult {
  round_id: number
  winning_square: number
  our_picks: number[]
  hit: boolean
  ore_earned: number
  timestamp: string
}

export async function GET() {
  const dbUrl = process.env.DATABASE_URL

  if (!dbUrl) {
    return NextResponse.json({ results: [], tally: { wins: 0, losses: 0, total: 0 } })
  }

  try {
    const { Pool } = await import('pg')
    const pool = new Pool({ connectionString: dbUrl, ssl: { rejectUnauthorized: false } })
    
    // Get recent round results with our picks
    const resultsQuery = await pool.query(`
      SELECT 
        r.round_id,
        r.winning_square,
        sp.recommended_squares as our_picks,
        sp.hit,
        COALESCE(wr.ore_earned, 0) as ore_earned,
        r.completed_at as timestamp
      FROM rounds r
      LEFT JOIN strategy_performance sp ON sp.round_id = r.round_id 
        AND sp.strategy_name = 'consensus'
      LEFT JOIN win_records wr ON wr.round_id = r.round_id 
        AND wr.winner_address LIKE 'Clawd%'
      WHERE r.winning_square IS NOT NULL
      ORDER BY r.round_id DESC
      LIMIT 50
    `)
    
    // Get overall tally
    const tallyQuery = await pool.query(`
      SELECT 
        COUNT(*) FILTER (WHERE hit = true) as wins,
        COUNT(*) FILTER (WHERE hit = false) as losses,
        COUNT(*) as total,
        COALESCE(SUM(CASE WHEN hit THEN 1 ELSE 0 END)::float / NULLIF(COUNT(*), 0) * 100, 0) as win_rate
      FROM strategy_performance
      WHERE strategy_name = 'consensus'
    `)
    
    // Get most recent winning square from rounds
    const lastWinnerQuery = await pool.query(`
      SELECT round_id, winning_square, completed_at
      FROM rounds
      WHERE winning_square IS NOT NULL
      ORDER BY round_id DESC
      LIMIT 1
    `)
    
    await pool.end()

    const results: RoundResult[] = resultsQuery.rows.map(row => ({
      round_id: row.round_id,
      winning_square: row.winning_square,
      our_picks: row.our_picks || [],
      hit: row.hit || false,
      ore_earned: parseFloat(row.ore_earned || '0'),
      timestamp: row.timestamp || new Date().toISOString(),
    }))

    const tally = tallyQuery.rows[0] || { wins: 0, losses: 0, total: 0, win_rate: 0 }
    const lastWinner = lastWinnerQuery.rows[0] || null

    return NextResponse.json({
      results,
      tally: {
        wins: parseInt(tally.wins || '0'),
        losses: parseInt(tally.losses || '0'),
        total: parseInt(tally.total || '0'),
        win_rate: parseFloat(tally.win_rate || '0').toFixed(1),
      },
      last_winner: lastWinner ? {
        round_id: lastWinner.round_id,
        winning_square: lastWinner.winning_square,
        timestamp: lastWinner.completed_at,
      } : null,
    })
  } catch (error) {
    console.error('Results API error:', error)
    return NextResponse.json({
      results: [],
      tally: { wins: 0, losses: 0, total: 0, win_rate: '0' },
      last_winner: null,
      error: String(error),
    })
  }
}
