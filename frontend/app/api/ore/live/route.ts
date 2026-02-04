import { NextResponse } from 'next/server'
import { Connection, PublicKey } from '@solana/web3.js'

// Force dynamic rendering
export const dynamic = 'force-dynamic'
export const revalidate = 0

// ORE Program ID
const ORE_PROGRAM_ID = new PublicKey('oreV3EG1i9BEgiAJ8b177Z2S2rMarzak4NMv1kULvWv')
const LAMPORTS_PER_SOL = 1_000_000_000

// RPC endpoint
const RPC_URL = process.env.SOLANA_RPC_URL || 'https://api.mainnet-beta.solana.com'

// Derive Board PDA
function getBoardPda(): PublicKey {
  const [pda] = PublicKey.findProgramAddressSync(
    [Buffer.from('board')],
    ORE_PROGRAM_ID
  )
  return pda
}

// Derive Round PDA
function getRoundPda(roundId: bigint): PublicKey {
  const roundIdBuffer = Buffer.alloc(8)
  roundIdBuffer.writeBigUInt64LE(roundId)
  const [pda] = PublicKey.findProgramAddressSync(
    [Buffer.from('round'), roundIdBuffer],
    ORE_PROGRAM_ID
  )
  return pda
}

// Parse Board account data (after 8-byte discriminator)
function parseBoard(data: Buffer): { round_id: bigint; start_slot: bigint; end_slot: bigint; epoch_id: bigint } {
  return {
    round_id: data.readBigUInt64LE(0),
    start_slot: data.readBigUInt64LE(8),
    end_slot: data.readBigUInt64LE(16),
    epoch_id: data.readBigUInt64LE(24),
  }
}

// Parse Round account data (after 8-byte discriminator)
function parseRound(data: Buffer) {
  // Round struct layout:
  // id: u64 (8)
  // deployed: [u64; 25] (200)
  // slot_hash: [u8; 32] (32)
  // count: [u64; 25] (200)
  // expires_at: u64 (8)
  // motherlode: u64 (8)
  // rent_payer: Pubkey (32)
  // top_miner: Pubkey (32)
  // top_miner_reward: u64 (8)
  // total_deployed: u64 (8)
  // total_miners: u64 (8)
  // total_vaulted: u64 (8)
  // total_winnings: u64 (8)
  
  const id = data.readBigUInt64LE(0)
  
  const deployed: bigint[] = []
  for (let i = 0; i < 25; i++) {
    deployed.push(data.readBigUInt64LE(8 + i * 8))
  }
  
  // slot_hash at offset 208 (8 + 200)
  const slotHash = data.slice(208, 240)
  
  // count at offset 240
  const count: bigint[] = []
  for (let i = 0; i < 25; i++) {
    count.push(data.readBigUInt64LE(240 + i * 8))
  }
  
  // expires_at at offset 440
  // motherlode at offset 448
  const motherlode = data.readBigUInt64LE(448)
  
  // rent_payer at offset 456 (32 bytes)
  // top_miner at offset 488 (32 bytes)
  const topMiner = new PublicKey(data.slice(488, 520))
  
  // top_miner_reward at offset 520
  const topMinerReward = data.readBigUInt64LE(520)
  
  // total_deployed at offset 528
  const totalDeployed = data.readBigUInt64LE(528)
  
  // total_miners at offset 536
  const totalMiners = data.readBigUInt64LE(536)
  
  // total_vaulted at offset 544
  const totalVaulted = data.readBigUInt64LE(544)
  
  return {
    id,
    deployed,
    slotHash,
    count,
    motherlode,
    topMiner,
    topMinerReward,
    totalDeployed,
    totalMiners,
    totalVaulted,
  }
}

export async function GET() {
  try {
    const connection = new Connection(RPC_URL, 'confirmed')
    
    // Get current slot
    const currentSlot = await connection.getSlot()
    
    // Get Board account
    const boardPda = getBoardPda()
    const boardAccount = await connection.getAccountInfo(boardPda)
    
    if (!boardAccount) {
      return NextResponse.json({ error: 'Board account not found' }, { status: 500 })
    }
    
    const board = parseBoard(boardAccount.data.slice(8)) // Skip 8-byte discriminator
    const roundId = board.round_id
    
    // Get Round account
    const roundPda = getRoundPda(roundId)
    const roundAccount = await connection.getAccountInfo(roundPda)
    
    if (!roundAccount) {
      return NextResponse.json({ error: 'Round account not found' }, { status: 500 })
    }
    
    const round = parseRound(roundAccount.data.slice(8)) // Skip 8-byte discriminator
    
    // Calculate timing
    const startSlot = Number(board.start_slot)
    const endSlot = Number(board.end_slot)
    const slotsRemaining = Math.max(0, endSlot - currentSlot)
    const isIntermission = currentSlot >= endSlot
    const timeRemainingSecs = Math.floor(slotsRemaining * 0.37) // ~370ms per slot
    
    // Calculate totals
    const totalDeployed = Number(round.totalDeployed)
    const totalMiners = Number(round.totalMiners)
    
    // Build square data
    const squares = round.deployed.map((deployed, i) => {
      const deployedNum = Number(deployed)
      const minerCount = Number(round.count[i])
      return {
        square_num: i + 1,
        index: i,
        deployed_lamports: deployedNum,
        deployed_sol: deployedNum / LAMPORTS_PER_SOL,
        miner_count: minerCount,
        is_winning: false,
        percentage_of_total: totalDeployed > 0 ? (deployedNum / totalDeployed) * 100 : 4.0,
      }
    })
    
    // Check for winning square if round completed
    const slotHashEmpty = round.slotHash.every((b: number) => b === 0)
    if (!slotHashEmpty && isIntermission) {
      // Calculate winning square from slot_hash RNG
      const view = new DataView(round.slotHash.buffer, round.slotHash.byteOffset)
      const rng = view.getBigUint64(0, true) ^ view.getBigUint64(8, true) ^ 
                  view.getBigUint64(16, true) ^ view.getBigUint64(24, true)
      const winningSquare = Number(rng % BigInt(25))
      if (winningSquare < 25) {
        squares[winningSquare].is_winning = true
      }
    }
    
    // Top miner
    const topMinerStr = round.topMiner.equals(PublicKey.default) ? null : round.topMiner.toBase58()
    
    return NextResponse.json({
      round_id: Number(roundId),
      start_slot: startSlot,
      end_slot: endSlot,
      current_slot: currentSlot,
      slots_remaining: slotsRemaining,
      time_remaining_secs: timeRemainingSecs,
      is_intermission: isIntermission,
      squares,
      total_deployed_lamports: totalDeployed,
      total_deployed_sol: totalDeployed / LAMPORTS_PER_SOL,
      total_miners: totalMiners,
      total_vaulted_lamports: Number(round.totalVaulted),
      total_vaulted_sol: Number(round.totalVaulted) / LAMPORTS_PER_SOL,
      top_miner: topMinerStr,
      top_miner_reward: Number(round.topMinerReward) / 1e11, // ORE has 11 decimals
      motherlode_lamports: Number(round.motherlode),
      motherlode_sol: Number(round.motherlode) / LAMPORTS_PER_SOL,
    })
  } catch (error) {
    console.error('Error fetching ORE live data:', error)
    return NextResponse.json({ 
      error: 'Failed to fetch live data',
      details: error instanceof Error ? error.message : 'Unknown error'
    }, { status: 500 })
  }
}
