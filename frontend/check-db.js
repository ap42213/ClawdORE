const { Client } = require('pg');

const client = new Client({
  connectionString: 'postgresql://postgres:TTWbmAztQQDuZZySvrbdJvQkfXfEoSEA@ballast.proxy.rlwy.net:12880/railway'
});

async function main() {
  await client.connect();
  console.log('Connected to Railway Postgres!\n');

  // Check rounds with winning_square
  const rounds = await client.query(`
    SELECT round_id, winning_square, completed_at 
    FROM rounds 
    ORDER BY round_id DESC 
    LIMIT 10
  `);
  console.log('=== Last 10 Rounds ===');
  rounds.rows.forEach(r => console.log(`  Round ${r.round_id}: winning_square=${r.winning_square}, completed=${r.completed_at}`));

  // Check Reset transactions
  const resets = await client.query(`
    SELECT round_id, data
    FROM transactions 
    WHERE instruction_type = 'Reset' 
    ORDER BY round_id DESC 
    LIMIT 3
  `);
  console.log('\n=== Reset Transactions (last 3) ===');
  resets.rows.forEach(r => console.log(`  Round ${r.round_id}: data=${JSON.stringify(r.data)}`));

  // Check strategy_performance
  const strats = await client.query(`
    SELECT * FROM strategy_performance ORDER BY round_id DESC LIMIT 5
  `);
  console.log('\n=== Strategy Performance ===');
  console.log(strats.rows);

  // Check square_count_stats
  const scs = await client.query(`
    SELECT * FROM square_count_stats WHERE times_won > 0 ORDER BY win_rate DESC LIMIT 5
  `);
  console.log('\n=== Square Count Stats (with wins) ===');
  console.log(scs.rows);

  await client.end();
}

main().catch(console.error);
