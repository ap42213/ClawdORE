import { NextResponse } from 'next/server'

// Force dynamic rendering
export const dynamic = 'force-dynamic'
export const revalidate = 0

export async function GET() {
  const dbUrl = process.env.DATABASE_URL

  if (!dbUrl) {
    // Return default analysis if no database
    const squares = Array.from({ length: 25 }, (_, i) => ({
      square_num: i + 1,
      total_deployed_sol: 0,
      times_won: 0,
      win_rate: 0.04,
      recommendation: '➖ NEUTRAL',
    }))
    return NextResponse.json({ squares, source: 'default' })
  }

  try {
    const { Pool } = await import('pg')
    const pool = new Pool({ connectionString: dbUrl, ssl: { rejectUnauthorized: false } })
    
    // Get square statistics from database
    const result = await pool.query(`
      SELECT 
        square_id,
        total_wins,
        total_rounds,
        total_deployed,
        win_rate,
        edge,
        streak,
        avg_competition
      FROM square_stats
      ORDER BY square_id
    `)
    
    // Also count rounds to calculate expected win rate
    const roundCountResult = await pool.query(`
      SELECT COUNT(*) as total_rounds FROM rounds WHERE winning_square IS NOT NULL
    `)
    const totalRounds = Number(roundCountResult.rows[0]?.total_rounds || 0)
    
    await pool.end()

    const expectedWinRate = 0.04 // 1/25 = 4%
    
    // Build squares array, filling in missing squares with defaults
    const squaresMap = new Map<number, any>()
    for (const row of result.rows) {
      const winRate = Number(row.win_rate) || 0.04
      let recommendation = '➖ NEUTRAL'
      
      if (winRate > expectedWinRate * 1.5) {
        recommendation = '🔥 HOT - Above expected'
      } else if (winRate < expectedWinRate * 0.5) {
        recommendation = '❄️ COLD - Below expected'
      }
      
      squaresMap.set(row.square_id, {
        square_num: row.square_id + 1, // Convert 0-based to 1-based
        total_deployed_sol: Number(row.total_deployed) / 1_000_000_000,
        times_won: row.total_wins,
        total_rounds: row.total_rounds,
        win_rate: winRate,
        edge: Number(row.edge) || 0,
        streak: row.streak || 0,
        avg_competition: Number(row.avg_competition) / 1_000_000_000,
        recommendation,
      })
    }

    // Fill in any missing squares
    const squares = Array.from({ length: 25 }, (_, i) => {
      if (squaresMap.has(i)) {
        return squaresMap.get(i)
      }
      return {
        square_num: i + 1,
        total_deployed_sol: 0,
        times_won: 0,
        total_rounds: 0,
        win_rate: 0.04,
        edge: 0,
        streak: 0,
        avg_competition: 0,
        recommendation: '➖ NEUTRAL',
      }
    })

    return NextResponse.json({
      squares,
      total_rounds_analyzed: totalRounds,
      source: 'database'
    })
  } catch (error) {
    console.error('Database error:', error)
    // Return default on error
    const squares = Array.from({ length: 25 }, (_, i) => ({
      square_num: i + 1,
      total_deployed_sol: 0,
      times_won: 0,
      win_rate: 0.04,
      recommendation: '➖ NEUTRAL',
    }))
    return NextResponse.json({ 
      squares, 
      source: 'error',
      error: String(error)
    })
  }
}
