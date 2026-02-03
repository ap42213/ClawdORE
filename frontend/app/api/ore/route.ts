import { NextResponse } from 'next/server'
import { Connection, PublicKey } from '@solana/web3.js'

// Force dynamic rendering - no caching
export const dynamic = 'force-dynamic'
export const revalidate = 0

// ORE Program ID
const ORE_PROGRAM_ID = 'oreV3EG1i9BEgiAJ8b177Z2S2rMarzak4NMv1kULvWv'

// Seeds
const BOARD_SEED = 'board'
const ROUND_SEED = 'round'

// Read u64 from buffer (little-endian)
function readU64(data: Buffer | Uint8Array, offset: number): bigint {
  let value = BigInt(0)
  for (let i = 0; i < 8; i++) {
    value += BigInt(data[offset + i]) << BigInt(i * 8)
  }
  return value
}

// Calculate winning square from slot_hash (same algorithm as ore-api)
// Returns 1-25 (ORE uses 1-indexed squares)
function calculateWinningSquare(slotHash: Uint8Array): number | null {
  // Check if slot_hash is unset (all zeros or all 255s)
  const allZeros = slotHash.every(b => b === 0)
  const allMax = slotHash.every(b => b === 255)
  if (allZeros || allMax) return null
  
  // XOR the 4 u64 values to get RNG
  const r1 = readU64(slotHash, 0)
  const r2 = readU64(slotHash, 8)
  const r3 = readU64(slotHash, 16)
  const r4 = readU64(slotHash, 24)
  const rng = r1 ^ r2 ^ r3 ^ r4
  
  // Winning square = (rng % 25) + 1 to get 1-25 range
  return Number(rng % BigInt(25)) + 1
}

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
    
    // Parse Board: discriminator(8) + round_id(8) + start_slot(8) + end_slot(8) + epoch_id(8)
    const boardData = boardAccount.data
    const roundId = Number(readU64(boardData, 8))
    const startSlot = Number(readU64(boardData, 16))
    const endSlot = Number(readU64(boardData, 24))
    
    // Get current slot
    const currentSlot = await connection.getSlot()
    const slotsRemaining = Math.max(0, endSlot - currentSlot)
    const timeRemainingSecs = slotsRemaining / 2.5
    
    // Derive Round PDA for current round
    const roundIdBuffer = Buffer.alloc(8)
    roundIdBuffer.writeBigUInt64LE(BigInt(roundId))
    const [currentRoundPda] = PublicKey.findProgramAddressSync(
      [Buffer.from(ROUND_SEED), roundIdBuffer],
      programId
    )
    
    // Fetch current round account
    const currentRoundAccount = await connection.getAccountInfo(currentRoundPda)
    
    let deployed: number[] = new Array(25).fill(0)
    let totalDeployed = 0
    
    // Round struct layout:
    // discriminator(8) + id(8) + deployed[25](200) + slot_hash[32](32) + count[25](200) + ...
    if (currentRoundAccount) {
      const roundData = currentRoundAccount.data
      // deployed array starts at offset 16 (after discriminator + id)
      for (let i = 0; i < 25; i++) {
        const value = Number(readU64(roundData, 16 + (i * 8)))
        deployed[i] = value
        totalDeployed += value
      }
    }
    
    // Fetch PREVIOUS round to get last winning square
    let lastWinningSquare: number | null = null
    let lastRoundId: number | null = null
    
    if (roundId > 1) {
      const prevRoundId = roundId - 1
      const prevRoundIdBuffer = Buffer.alloc(8)
      prevRoundIdBuffer.writeBigUInt64LE(BigInt(prevRoundId))
      const [prevRoundPda] = PublicKey.findProgramAddressSync(
        [Buffer.from(ROUND_SEED), prevRoundIdBuffer],
        programId
      )
      
      const prevRoundAccount = await connection.getAccountInfo(prevRoundPda)
      if (prevRoundAccount) {
        // slot_hash is at offset 16 + 200 = 216 (after discriminator + id + deployed[25])
        const slotHashOffset = 16 + (25 * 8) // 216
        const slotHash = prevRoundAccount.data.slice(slotHashOffset, slotHashOffset + 32)
        lastWinningSquare = calculateWinningSquare(slotHash)
        lastRoundId = prevRoundId
      }
    }
    
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
      last_winning_square: lastWinningSquare,
      last_round_id: lastRoundId,
    })
  } catch (error) {
    console.error('ORE fetch error:', error)
    return NextResponse.json({ error: String(error) })
  }
}
