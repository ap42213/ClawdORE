import { NextResponse } from 'next/server'

export async function GET() {
  const dbUrl = process.env.DATABASE_URL

  if (!dbUrl) {
    return NextResponse.json({})
  }

  try {
    const { Pool } = await import('pg')
    const pool = new Pool({ connectionString: dbUrl, ssl: { rejectUnauthorized: false } })
    
    const result = await pool.query(`
      SELECT key, value, updated_at
      FROM bot_state
    `)
    
    await pool.end()

    // Convert to object
    const state: Record<string, any> = {}
    for (const row of result.rows) {
      state[row.key] = row.value
    }

    return NextResponse.json(state)
  } catch (error) {
    console.error('Database error:', error)
    return NextResponse.json({ error: String(error) })
  }
}
