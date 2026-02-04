import { NextResponse } from 'next/server'

// Force dynamic rendering - no caching
export const dynamic = 'force-dynamic'
export const revalidate = 0

export async function GET() {
  const dbUrl = process.env.DATABASE_URL

  if (!dbUrl) {
    return NextResponse.json({
      total: 0,
      hits: 0,
      misses: 0,
      win_rate: 0,
      baseline: 80,
      edge: 0,
      recent_results: [],
      error: 'No database configured'
    })
  }

  try {
    const { Pool } = await import('pg')
    const pool = new Pool({ connectionString: dbUrl, ssl: { rejectUnauthorized: false } })
    
    // Get total and hits
    const [total, hits, recent] = await Promise.all([
      pool.query(`SELECT COUNT(*) FROM test_20_rounds WHERE is_hit IS NOT NULL`),
      pool.query(`SELECT COUNT(*) FROM test_20_rounds WHERE is_hit = true`),
      pool.query(`
        SELECT 
          round_id, 
          COALESCE(winning_square, 0) as winning_square, 
          COALESCE(is_hit, false) as is_hit, 
          betting_squares, 
          skipping_squares,
          completed_at
        FROM test_20_rounds 
        WHERE is_hit IS NOT NULL 
        ORDER BY completed_at DESC 
        LIMIT 100
      `),
    ])
    
    await pool.end()

    const totalCount = parseInt(total.rows[0]?.count || '0')
    const hitsCount = parseInt(hits.rows[0]?.count || '0')
    const missesCount = totalCount - hitsCount
    const winRate = totalCount > 0 ? (hitsCount / totalCount * 100) : 0
    const baseline = 80
    const edge = winRate - baseline

    const recentResults = recent.rows.map(row => ({
      round_id: parseInt(row.round_id),
      winning_square: parseInt(row.winning_square),
      is_hit: row.is_hit,
      betting_squares: row.betting_squares,
      skipping_squares: row.skipping_squares,
      completed_at: row.completed_at
    }))

    return NextResponse.json({
      total: totalCount,
      hits: hitsCount,
      misses: missesCount,
      win_rate: parseFloat(winRate.toFixed(2)),
      baseline,
      edge: parseFloat(edge.toFixed(2)),
      recent_results: recentResults
    })
  } catch (error) {
    console.error('Test-20 API error:', error)
    return NextResponse.json({
      total: 0,
      hits: 0,
      misses: 0,
      win_rate: 0,
      baseline: 80,
      edge: 0,
      recent_results: [],
      error: String(error)
    })
  }
}
