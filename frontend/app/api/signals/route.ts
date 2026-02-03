import { NextResponse } from 'next/server'

export async function GET() {
  const dbUrl = process.env.DATABASE_URL

  if (!dbUrl) {
    return NextResponse.json({ signals: [] })
  }

  try {
    // Use pg library to query database
    const { Pool } = await import('pg')
    const pool = new Pool({ connectionString: dbUrl, ssl: { rejectUnauthorized: false } })
    
    const result = await pool.query(`
      SELECT id, signal_type, source_bot, target_bot, payload, processed, created_at
      FROM signals
      ORDER BY created_at DESC
      LIMIT 50
    `)
    
    await pool.end()

    return NextResponse.json({ signals: result.rows })
  } catch (error) {
    console.error('Database error:', error)
    return NextResponse.json({ signals: [], error: String(error) })
  }
}
