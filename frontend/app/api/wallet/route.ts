import { NextResponse } from 'next/server'

const AGENTWALLET_USERNAME = 'ap42213'
const AGENTWALLET_TOKEN = process.env.AGENTWALLET_TOKEN || 'mf_0934d44ebe0aae96c20e502c1509945d0f5cbebe5a68fd7ac86007048bd234fc'
const SOLANA_ADDRESS = '3cTXnSVQwyU3GdTwQgiTfocVgRM9mmSYJ3BrLfADvHrV'

export async function GET() {
  try {
    const response = await fetch(
      `https://agentwallet.mcpay.tech/api/wallets/${AGENTWALLET_USERNAME}/balances`,
      {
        headers: {
          'Authorization': `Bearer ${AGENTWALLET_TOKEN}`,
        },
        next: { revalidate: 30 } // Cache for 30 seconds
      }
    )

    if (!response.ok) {
      throw new Error(`AgentWallet API error: ${response.status}`)
    }

    const data = await response.json()
    
    // Extract Solana balances
    const solanaWallet = data.solana || data.solanaWallets?.[0]
    const solBalance = solanaWallet?.balances?.find((b: any) => b.asset === 'sol')
    const usdcBalance = solanaWallet?.balances?.find((b: any) => b.asset === 'usdc')

    return NextResponse.json({
      address: SOLANA_ADDRESS,
      sol: solBalance ? Number(solBalance.rawValue) / 1e9 : 0,
      usdc: usdcBalance ? Number(usdcBalance.rawValue) / 1e6 : 0,
      username: AGENTWALLET_USERNAME,
    })
  } catch (error) {
    console.error('Wallet fetch error:', error)
    return NextResponse.json({
      address: SOLANA_ADDRESS,
      sol: 0,
      usdc: 0,
      username: AGENTWALLET_USERNAME,
      error: 'Failed to fetch balance'
    })
  }
}
