const { Client } = require('pg');

const client = new Client({
  connectionString: 'postgresql://postgres:TTWbmAztQQDuZZySvrbdJvQkfXfEoSEA@ballast.proxy.rlwy.net:12880/railway'
});

async function main() {
  await client.connect();
  console.log('Connected to Railway Postgres!\n');

  // List tables
  const tables = await client.query("SELECT table_name FROM information_schema.tables WHERE table_schema = 'public'");
  console.log('📋 Tables:');
  tables.rows.forEach(r => console.log('  -', r.table_name));

  // Count signals
  try {
    const signals = await client.query('SELECT COUNT(*) as count FROM signals');
    console.log('\n📤 Signals count:', signals.rows[0].count);
    
    const recent = await client.query('SELECT signal_type, source_bot, created_at FROM signals ORDER BY created_at DESC LIMIT 5');
    console.log('\n📤 Recent signals:');
    recent.rows.forEach(r => console.log(`  - ${r.signal_type} from ${r.source_bot} at ${r.created_at}`));
  } catch (e) {
    console.log('No signals table yet');
  }

  // Count rounds
  try {
    const rounds = await client.query('SELECT COUNT(*) as count FROM rounds');
    console.log('\n🔄 Rounds count:', rounds.rows[0].count);
  } catch (e) {
    console.log('No rounds table yet');
  }

  await client.end();
}

main().catch(console.error);
