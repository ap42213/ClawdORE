import { NextResponse } from 'next/server'

// Force dynamic rendering
export const dynamic = 'force-dynamic'
export const revalidate = 0

export async function GET() {
  const dbUrl = process.env.DATABASE_URL

  if (!dbUrl) {
    return NextResponse.json({
      total_rounds_completed: 0,
      total_sol_deployed: 0,
      total_motherlode_rounds: 0,
      motherlode_rate: 0,
      avg_deployment_per_round: 0,
      source: 'no-database'
    })
  }

  try {
    const { Pool } = await import('pg')
    const pool = new Pool({ connectionString: dbUrl, ssl: { rejectUnauthorized: false } })
    
    // Get aggregated protocol stats from rounds table
    const statsResult = await pool.query(`
      SELECT 
        COUNT(*) as total_rounds,
        SUM(total_deployed) as total_deployed,
        COUNT(*) FILTER (WHERE motherlode = true) as motherlode_rounds,
        AVG(total_deployed) as avg_deployment
      FROM rounds
      WHERE winning_square IS NOT NULL
    `)
    
    // Get bot performance stats if available
    const botStatsResult = await pool.query(`
      SELECT 
        COUNT(DISTINCT strategy_name) as unique_strategies,
        SUM(CASE WHEN hit THEN 1 ELSE 0 END) as total_hits,
        COUNT(*) as total_predictions,
        AVG(confidence) as avg_confidence
      FROM strategy_performance
    `)
    
    // Get whale count
    const whaleResult = await pool.query(`
      SELECT COUNT(*) as whale_count FROM whales
    `)
    
    await pool.end()

    const stats = statsResult.rows[0]
    const botStats = botStatsResult.rows[0]
    const whaleCount = Number(whaleResult.rows[0]?.whale_count || 0)
    
    const totalRounds = Number(stats?.total_rounds || 0)
    const totalDeployed = Number(stats?.total_deployed || 0)
    const motherlodeRounds = Number(stats?.motherlode_rounds || 0)

    return NextResponse.json({
      total_rounds_completed: totalRounds,
      total_sol_deployed: totalDeployed / 1_000_000_000,
      total_motherlode_rounds: motherlodeRounds,
      motherlode_rate: totalRounds > 0 ? motherlodeRounds / totalRounds : 0,
      avg_deployment_per_round: totalRounds > 0 ? (totalDeployed / totalRounds) / 1_000_000_000 : 0,
      tracked_whales: whaleCount,
      bot_stats: {
        unique_strategies: Number(botStats?.unique_strategies || 0),
        total_predictions: Number(botStats?.total_predictions || 0),
        total_hits: Number(botStats?.total_hits || 0),
        hit_rate: Number(botStats?.total_predictions) > 0 
          ? Number(botStats?.total_hits) / Number(botStats?.total_predictions) 
          : 0,
        avg_confidence: Number(botStats?.avg_confidence || 0),
      },
      source: 'database'
    })
  } catch (error) {
    console.error('Protocol stats error:', error)
    return NextResponse.json({
      total_rounds_completed: 0,
      total_sol_deployed: 0,
      total_motherlode_rounds: 0,
      motherlode_rate: 0,
      avg_deployment_per_round: 0,
      source: 'error',
      error: String(error)
    })
  }
}
