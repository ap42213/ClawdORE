const { Pool } = require('pg');

const pool = new Pool({ 
  connectionString: process.env.DATABASE_URL, 
  ssl: { rejectUnauthorized: false } 
});

async function main() {
  try {
    // Check betting bot signals
    const result = await pool.query(`
      SELECT signal_type, payload, created_at 
      FROM signals 
      WHERE source_bot = 'betting-bot' 
      ORDER BY created_at DESC 
      LIMIT 10
    `);
    
    console.log('=== Betting Bot Activity ===');
    result.rows.forEach(row => {
      console.log(row.created_at.toISOString(), '-', row.signal_type);
      if (row.payload && row.signal_type === 'bet_placed') {
        console.log('  Signature:', row.payload.signature);
        console.log('  Squares:', row.payload.squares);
        console.log('  Total:', row.payload.total_bet_sol, 'SOL');
      }
    });
    
    // Check if any bet_placed signals exist
    const bets = await pool.query(`
      SELECT COUNT(*) as count FROM signals 
      WHERE source_bot = 'betting-bot' AND signal_type = 'bet_placed'
    `);
    console.log('\nTotal bet_placed signals:', bets.rows[0].count);
    
  } catch (e) {
    console.error(e);
  } finally {
    await pool.end();
  }
}

main();
