const { Pool } = require('pg');

const pool = new Pool({ 
  connectionString: 'postgresql://postgres:TTWbmAztQQDuZZySvrbdJvQkfXfEoSEA@ballast.proxy.rlwy.net:12880/railway', 
  ssl: { rejectUnauthorized: false } 
});

async function check() {
  const result = await pool.query("SELECT key, value FROM bot_state WHERE key = 'consensus_recommendation'");
  console.log('Consensus Recommendation:');
  console.log(JSON.stringify(result.rows[0]?.value, null, 2));
  
  const strats = await pool.query("SELECT key, value FROM bot_state WHERE key = 'current_strategies'");
  console.log('\nCurrent Strategies:');
  console.log(JSON.stringify(strats.rows[0]?.value, null, 2));
  
  await pool.end();
}

check().catch(e => { console.error(e); pool.end(); });
