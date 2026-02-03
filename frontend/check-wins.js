const { Pool } = require('pg');

const pool = new Pool({ 
  connectionString: 'postgresql://postgres:TTWbmAztQQDuZZySvrbdJvQkfXfEoSEA@ballast.proxy.rlwy.net:12880/railway', 
  ssl: { rejectUnauthorized: false } 
});

async function check() {
  // Check Reset transactions
  const resets = await pool.query("SELECT * FROM transactions WHERE instruction_type = 'Reset' ORDER BY block_time DESC");
  console.log('Reset transactions found:', resets.rows.length);
  resets.rows.forEach(row => {
    console.log('\nReset TX:', row.signature);
    console.log('  Slot:', row.slot);
    console.log('  Block time:', row.block_time);
    console.log('  Signer:', row.signer);
    console.log('  Round ID:', row.round_id);
  });

  // Check if rounds have winning squares
  const roundsWithWins = await pool.query("SELECT round_id, winning_square FROM rounds WHERE winning_square IS NOT NULL LIMIT 5");
  console.log('\nRounds with winners:', roundsWithWins.rows.length);

  await pool.end();
}

check().catch(e => { console.error(e); pool.end(); });
