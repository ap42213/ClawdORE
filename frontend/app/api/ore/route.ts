import { NextResponse } from 'next/server'
import { Connection, PublicKey } from '@solana/web3.js'

// Force dynamic rendering - no caching
export const dynamic = 'force-dynamic'
export const revalidate = 0

// ORE Program ID
const ORE_PROGRAM_ID = 'oreV3EG1i9BEgiAJ8b177Z2S2rMarzak4NMv1kULvWv'

// Board PDA seed
const BOARD_SEED = 'board'

export async function GET() {
  const rpcUrl = process.env.RPC_URL || 'https://api.mainnet-beta.solana.com'

  try {
    const connection = new Connection(rpcUrl, 'confirmed')
    const programId = new PublicKey(ORE_PROGRAM_ID)
    
    // Derive Board PDA
    const [boardPda] = PublicKey.findProgramAddressSync(
      [Buffer.from(BOARD_SEED)],
      programId
    )
    
    // Fetch board account
    const boardAccount = await connection.getAccountInfo(boardPda)
    
    if (!boardAccount) {
      return NextResponse.json({ error: 'Board account not found' })
    }
    
    // Parse board data (simplified - actual parsing depends on ore_api struct layout)
    // Board struct: round_id (u64), start_slot (u64), end_slot (u64), etc.
    const data = boardAccount.data
    
    // Read u64 values (little-endian)
    const readU64 = (offset: number) => {
      let value = BigInt(0)
      for (let i = 0; i < 8; i++) {
        value += BigInt(data[offset + i]) << BigInt(i * 8)
      }
      return Number(value)
    }
    
    // Skip 8-byte discriminator
    const roundId = readU64(8)
    const startSlot = readU64(16)
    const endSlot = readU64(24)
    
    // Get current slot
    const currentSlot = await connection.getSlot()
    const slotsRemaining = Math.max(0, endSlot - currentSlot)
    
    // Estimate time remaining (Solana ~400ms per slot, but often faster ~350ms)
    const timeRemainingSecs = slotsRemaining / 2.5
    
    // Derive Round PDA for current round
    const [roundPda] = PublicKey.findProgramAddressSync(
      [Buffer.from('round'), Buffer.from(new BigUint64Array([BigInt(roundId)]).buffer)],
      programId
    )
    
    // Fetch round account
    const roundAccount = await connection.getAccountInfo(roundPda)
    
    let deployed: number[] = new Array(25).fill(0)
    let totalDeployed = 0
    
    if (roundAccount) {
      // Parse deployed array (25 u64 values after discriminator)
      // This is simplified - actual layout may vary
      for (let i = 0; i < 25; i++) {
        const offset = 8 + (i * 8) // Skip discriminator
        deployed[i] = readU64.call({ data: roundAccount.data }, offset)
        totalDeployed += deployed[i]
      }
    }
    
    // Determine winning square from previous round (if available)
    // This would require parsing the last Reset transaction
    
    return NextResponse.json({
      round_id: roundId,
      start_slot: startSlot,
      end_slot: endSlot,
      current_slot: currentSlot,
      slots_remaining: slotsRemaining,
      time_remaining_secs: Math.round(timeRemainingSecs),
      deployed: deployed.map(d => d / 1_000_000_000), // Convert to SOL
      total_deployed_sol: totalDeployed / 1_000_000_000,
      active_squares: deployed.filter(d => d > 0).length,
    })
  } catch (error) {
    console.error('ORE fetch error:', error)
    return NextResponse.json({ error: String(error) })
  }
}
