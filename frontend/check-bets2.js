const { Pool } = require('pg');

const pool = new Pool({ 
  connectionString: process.env.DATABASE_URL, 
  ssl: { rejectUnauthorized: false } 
});

async function main() {
  try {
    // Get the actual bet that was placed
    const bets = await pool.query(`
      SELECT signal_type, payload, created_at 
      FROM signals 
      WHERE source_bot = 'betting-bot' AND signal_type = 'bet_placed'
      ORDER BY created_at DESC 
      LIMIT 5
    `);
    
    console.log('=== Actual Bets Placed ===');
    bets.rows.forEach(row => {
      console.log('\nTime:', row.created_at.toISOString());
      console.log('Payload:', JSON.stringify(row.payload, null, 2));
    });
    
    // Get recent errors
    const errors = await pool.query(`
      SELECT payload, created_at 
      FROM signals 
      WHERE source_bot = 'betting-bot' AND signal_type = 'error'
      ORDER BY created_at DESC 
      LIMIT 3
    `);
    
    console.log('\n=== Recent Errors ===');
    errors.rows.forEach(row => {
      console.log('\nTime:', row.created_at.toISOString());
      console.log('Error:', JSON.stringify(row.payload, null, 2));
    });
    
  } catch (e) {
    console.error(e);
  } finally {
    await pool.end();
  }
}

main();
