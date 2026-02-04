import { NextResponse } from 'next/server'

// Force dynamic rendering
export const dynamic = 'force-dynamic'
export const revalidate = 0

export async function GET() {
  const dbUrl = process.env.DATABASE_URL

  if (!dbUrl) {
    return NextResponse.json({ rounds: [], count: 0, error: 'No database configured' })
  }

  try {
    const { Pool } = await import('pg')
    const pool = new Pool({ connectionString: dbUrl, ssl: { rejectUnauthorized: false } })
    
    // Get recent completed rounds from database
    const result = await pool.query(`
      SELECT 
        round_id,
        winning_square,
        total_deployed,
        total_vaulted,
        motherlode,
        num_deploys,
        deployed_squares,
        completed_at
      FROM rounds
      WHERE winning_square IS NOT NULL
      ORDER BY round_id DESC
      LIMIT 50
    `)
    
    await pool.end()

    const rounds = result.rows.map(row => ({
      round_id: Number(row.round_id),
      winning_square: row.winning_square,
      total_deployed_sol: Number(row.total_deployed) / 1_000_000_000,
      total_vaulted_sol: Number(row.total_vaulted) / 1_000_000_000,
      total_miners: row.num_deploys || 0,
      is_motherlode: row.motherlode || false,
      deployed_squares: row.deployed_squares || [],
      completed_at: row.completed_at,
    }))

    return NextResponse.json({
      rounds,
      count: rounds.length,
      source: 'database'
    })
  } catch (error) {
    console.error('Database error:', error)
    return NextResponse.json({ 
      rounds: [], 
      count: 0, 
      error: String(error),
      source: 'error'
    })
  }
}
