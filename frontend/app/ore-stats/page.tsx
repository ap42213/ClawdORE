'use client'

import { useState, useEffect } from 'react'
import Link from 'next/link'

// Types matching the Rust backend
interface SquareData {
  square_num: number
  index: number
  deployed_lamports: number
  deployed_sol: number
  miner_count: number
  is_winning: boolean
  percentage_of_total: number
}

interface LiveRoundData {
  round_id: number
  start_slot: number
  end_slot: number
  current_slot: number
  slots_remaining: number
  time_remaining_secs: number
  is_intermission: boolean
  squares: SquareData[]
  total_deployed_lamports: number
  total_deployed_sol: number
  total_miners: number
  total_vaulted_lamports: number
  total_vaulted_sol: number
  top_miner: string | null
  top_miner_reward: number | null
  motherlode_lamports: number
  motherlode_sol: number
}

interface ProtocolStats {
  treasury_balance_sol?: number
  motherlode_sol?: number
  total_staked_ore?: number
  ore_price_usd?: number | null
  sol_price_usd?: number | null
  // Database-sourced stats
  total_rounds_completed?: number
  total_sol_deployed?: number
  total_motherlode_rounds?: number
  motherlode_rate?: number
  avg_deployment_per_round?: number
  tracked_whales?: number
  bot_stats?: {
    unique_strategies: number
    total_predictions: number
    total_hits: number
    hit_rate: number
    avg_confidence: number
  }
  source?: string
}

interface RoundHistory {
  round_id: number
  total_deployed_sol: number
  total_vaulted_sol: number
  total_miners: number
  winning_square: number
  is_motherlode: boolean
  top_miner: string
  top_miner_reward_ore: number
}

interface SquareAnalysis {
  square_num: number
  times_won: number
  total_rounds?: number
  win_rate: number
  edge?: number
  streak?: number
  avg_competition?: number
  recommendation: string
}

export default function OreStatsPage() {
  const [liveRound, setLiveRound] = useState<LiveRoundData | null>(null)
  const [protocol, setProtocol] = useState<ProtocolStats | null>(null)
  const [history, setHistory] = useState<RoundHistory[]>([])
  const [analysis, setAnalysis] = useState<SquareAnalysis[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [lastUpdate, setLastUpdate] = useState<Date | null>(null)

  // Use local API routes (they'll fetch from Solana directly)
  const fetchData = async () => {
    try {
      const [liveRes, protocolRes, historyRes, analysisRes] = await Promise.all([
        fetch('/api/ore/live'),
        fetch('/api/ore/protocol'),
        fetch('/api/ore/history'),
        fetch('/api/ore/squares'),
      ])

      if (liveRes.ok) {
        const data = await liveRes.json()
        if (!data.error) setLiveRound(data)
        else setError(data.error)
      }
      
      if (protocolRes.ok) {
        const data = await protocolRes.json()
        if (!data.error) setProtocol(data)
      }
      
      if (historyRes.ok) {
        const data = await historyRes.json()
        if (!data.error && data.rounds) setHistory(data.rounds)
      }
      
      if (analysisRes.ok) {
        const data = await analysisRes.json()
        if (!data.error && data.squares) setAnalysis(data.squares)
      }

      setLastUpdate(new Date())
      setError(null)
    } catch (e) {
      setError('Failed to fetch ORE data')
      console.error(e)
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    fetchData()
    const interval = setInterval(fetchData, 5000) // Refresh every 5 seconds
    return () => clearInterval(interval)
  }, [])

  const formatSol = (sol: number) => sol.toFixed(4)
  const formatPercent = (pct: number) => pct.toFixed(1)
  const shortenAddress = (addr: string) => addr ? `${addr.slice(0, 4)}...${addr.slice(-4)}` : ''

  // Get color intensity based on deployment
  const getSquareColor = (sq: SquareData, maxDeployed: number) => {
    if (sq.is_winning) return 'bg-yellow-500 text-black'
    if (maxDeployed === 0) return 'bg-gray-800'
    
    const intensity = sq.deployed_sol / maxDeployed
    if (intensity > 0.15) return 'bg-purple-600'
    if (intensity > 0.08) return 'bg-purple-700'
    if (intensity > 0.04) return 'bg-purple-800'
    return 'bg-gray-800'
  }

  if (loading) {
    return (
      <div className="min-h-screen bg-gradient-to-br from-gray-900 via-purple-900/20 to-gray-900 flex items-center justify-center">
        <div className="text-purple-400 text-xl animate-pulse">Loading ORE Stats...</div>
      </div>
    )
  }

  const maxDeployed = liveRound ? Math.max(...liveRound.squares.map(s => s.deployed_sol)) : 0

  return (
    <div className="min-h-screen bg-gradient-to-br from-gray-900 via-purple-900/20 to-gray-900 text-white p-4">
      {/* Header */}
      <div className="max-w-7xl mx-auto">
        <div className="flex items-center justify-between mb-6">
          <div>
            <h1 className="text-3xl font-bold text-purple-400">🦀 ClawdORE Stats</h1>
            <p className="text-gray-400 text-sm">Live ORE Mining Data from Solana</p>
          </div>
          <div className="flex items-center gap-4">
            <Link href="/" className="text-purple-400 hover:text-purple-300 text-sm">
              ← Back to Dashboard
            </Link>
            {lastUpdate && (
              <span className="text-xs text-gray-500">
                Updated: {lastUpdate.toLocaleTimeString()}
              </span>
            )}
          </div>
        </div>

        {error && (
          <div className="bg-red-900/30 border border-red-600 rounded-lg p-4 mb-6">
            <p className="text-red-400">{error}</p>
          </div>
        )}

        {/* Live Round Section */}
        {liveRound && (
          <div className="grid grid-cols-1 lg:grid-cols-3 gap-6 mb-8">
            {/* 5x5 Grid */}
            <div className="lg:col-span-2 bg-gray-800/50 rounded-xl p-6 border border-gray-700">
              <div className="flex items-center justify-between mb-4">
                <h2 className="text-xl font-bold text-purple-400">
                  Round #{liveRound.round_id}
                </h2>
                <div className={`px-3 py-1 rounded-full text-sm font-bold ${
                  liveRound.is_intermission 
                    ? 'bg-yellow-600 text-black' 
                    : 'bg-green-600 text-white'
                }`}>
                  {liveRound.is_intermission ? '⏸️ INTERMISSION' : '🔴 LIVE'}
                </div>
              </div>

              {/* Grid */}
              <div className="grid grid-cols-5 gap-2 mb-6">
                {liveRound.squares.map((sq) => (
                  <div
                    key={sq.square_num}
                    className={`aspect-square rounded-lg flex flex-col items-center justify-center p-2 transition-all ${getSquareColor(sq, maxDeployed)} ${sq.is_winning ? 'ring-4 ring-yellow-400 animate-pulse' : ''}`}
                  >
                    <span className="text-lg font-bold">{sq.square_num}</span>
                    <span className="text-xs opacity-80">{formatSol(sq.deployed_sol)} SOL</span>
                    <span className="text-xs opacity-60">{sq.miner_count} miners</span>
                  </div>
                ))}
              </div>

              {/* Time remaining */}
              <div className="flex items-center justify-center gap-8 text-center">
                <div>
                  <div className="text-3xl font-bold text-yellow-400">
                    {Math.floor(liveRound.time_remaining_secs / 60)}:{(liveRound.time_remaining_secs % 60).toString().padStart(2, '0')}
                  </div>
                  <div className="text-xs text-gray-400">Time Remaining</div>
                </div>
                <div>
                  <div className="text-2xl font-bold text-purple-400">{liveRound.slots_remaining}</div>
                  <div className="text-xs text-gray-400">Slots</div>
                </div>
              </div>
            </div>

            {/* Round Stats */}
            <div className="bg-gray-800/50 rounded-xl p-6 border border-gray-700">
              <h2 className="text-lg font-bold text-purple-400 mb-4">Round Statistics</h2>
              
              <div className="space-y-4">
                <div className="flex justify-between">
                  <span className="text-gray-400">Total Deployed</span>
                  <span className="font-bold text-green-400">{formatSol(liveRound.total_deployed_sol)} SOL</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Unique Miners</span>
                  <span className="font-bold">{liveRound.total_miners}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Vaulted</span>
                  <span className="font-bold text-blue-400">{formatSol(liveRound.total_vaulted_sol)} SOL</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Motherlode</span>
                  <span className="font-bold text-yellow-400">{formatSol(liveRound.motherlode_sol)} SOL</span>
                </div>
                
                {liveRound.top_miner && (
                  <div className="pt-4 border-t border-gray-700">
                    <span className="text-gray-400 text-sm">Top Miner</span>
                    <div className="font-mono text-xs text-purple-400 break-all">
                      {shortenAddress(liveRound.top_miner)}
                    </div>
                    {liveRound.top_miner_reward && (
                      <div className="text-yellow-400 text-sm">
                        +{liveRound.top_miner_reward.toFixed(4)} ORE
                      </div>
                    )}
                  </div>
                )}
              </div>
            </div>
          </div>
        )}

        {/* Protocol Stats */}
        {protocol && (
          <div className="bg-gray-800/50 rounded-xl p-6 border border-gray-700 mb-8">
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-lg font-bold text-purple-400">Protocol Stats</h2>
              {protocol.source && (
                <span className={`text-xs px-2 py-1 rounded ${
                  protocol.source === 'database' ? 'bg-green-900 text-green-400' : 'bg-gray-700 text-gray-400'
                }`}>
                  {protocol.source === 'database' ? '✓ From Database' : 'No Data'}
                </span>
              )}
            </div>
            <div className="grid grid-cols-2 md:grid-cols-4 gap-6 mb-6">
              <div>
                <div className="text-2xl font-bold text-green-400">{protocol.total_rounds_completed || 0}</div>
                <div className="text-xs text-gray-400">Rounds Tracked</div>
              </div>
              <div>
                <div className="text-2xl font-bold text-yellow-400">{formatSol(protocol.total_sol_deployed || 0)} SOL</div>
                <div className="text-xs text-gray-400">Total SOL Deployed</div>
              </div>
              <div>
                <div className="text-2xl font-bold text-purple-400">{protocol.total_motherlode_rounds || 0}</div>
                <div className="text-xs text-gray-400">Motherlode Rounds</div>
              </div>
              <div>
                <div className="text-2xl font-bold text-cyan-400">{formatPercent((protocol.motherlode_rate || 0) * 100)}%</div>
                <div className="text-xs text-gray-400">Motherlode Rate</div>
              </div>
            </div>
            
            {/* Bot Performance */}
            {protocol.bot_stats && protocol.bot_stats.total_predictions > 0 && (
              <div className="border-t border-gray-700 pt-4">
                <h3 className="text-sm font-bold text-purple-400 mb-3">🤖 Bot Learning Stats</h3>
                <div className="grid grid-cols-2 md:grid-cols-5 gap-4">
                  <div>
                    <div className="text-xl font-bold text-blue-400">{protocol.bot_stats.unique_strategies}</div>
                    <div className="text-xs text-gray-400">Strategies</div>
                  </div>
                  <div>
                    <div className="text-xl font-bold text-green-400">{protocol.bot_stats.total_predictions}</div>
                    <div className="text-xs text-gray-400">Predictions</div>
                  </div>
                  <div>
                    <div className="text-xl font-bold text-yellow-400">{protocol.bot_stats.total_hits}</div>
                    <div className="text-xs text-gray-400">Hits</div>
                  </div>
                  <div>
                    <div className="text-xl font-bold text-purple-400">{formatPercent(protocol.bot_stats.hit_rate * 100)}%</div>
                    <div className="text-xs text-gray-400">Hit Rate</div>
                  </div>
                  <div>
                    <div className="text-xl font-bold text-cyan-400">{formatPercent(protocol.bot_stats.avg_confidence * 100)}%</div>
                    <div className="text-xs text-gray-400">Avg Confidence</div>
                  </div>
                </div>
              </div>
            )}
            
            {protocol.tracked_whales !== undefined && protocol.tracked_whales > 0 && (
              <div className="mt-4 text-sm text-gray-400">
                🐋 Tracking {protocol.tracked_whales} whales
              </div>
            )}
          </div>
        )}

        {/* Recent Rounds & Square Analysis */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {/* Recent Rounds */}
          <div className="bg-gray-800/50 rounded-xl p-6 border border-gray-700">
            <h2 className="text-lg font-bold text-purple-400 mb-4">Recent Rounds</h2>
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr className="text-gray-400 border-b border-gray-700">
                    <th className="text-left py-2">Round</th>
                    <th className="text-right py-2">Deployed</th>
                    <th className="text-center py-2">Winner</th>
                    <th className="text-right py-2">Miners</th>
                  </tr>
                </thead>
                <tbody>
                  {history.slice(0, 10).map((round) => (
                    <tr key={round.round_id} className="border-b border-gray-800">
                      <td className="py-2">
                        #{round.round_id}
                        {round.is_motherlode && <span className="ml-1">💎</span>}
                      </td>
                      <td className="text-right text-green-400">{formatSol(round.total_deployed_sol)}</td>
                      <td className="text-center">
                        <span className="bg-yellow-600 text-black px-2 py-0.5 rounded text-xs font-bold">
                          {round.winning_square}
                        </span>
                      </td>
                      <td className="text-right text-gray-400">{round.total_miners}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>

          {/* Square Analysis */}
          <div className="bg-gray-800/50 rounded-xl p-6 border border-gray-700">
            <h2 className="text-lg font-bold text-purple-400 mb-4">Square Analysis (Last 100 Rounds)</h2>
            <div className="grid grid-cols-5 gap-2">
              {analysis.map((sq) => {
                const isHot = sq.recommendation.includes('HOT')
                const isCold = sq.recommendation.includes('COLD')
                return (
                  <div
                    key={sq.square_num}
                    className={`aspect-square rounded-lg flex flex-col items-center justify-center p-1 text-xs ${
                      isHot ? 'bg-red-600' : isCold ? 'bg-blue-600' : 'bg-gray-700'
                    }`}
                    title={sq.recommendation}
                  >
                    <span className="font-bold">{sq.square_num}</span>
                    <span className="opacity-80">{sq.times_won}W</span>
                    <span className="opacity-60">{formatPercent(sq.win_rate * 100)}%</span>
                  </div>
                )
              })}
            </div>
            <div className="flex justify-center gap-4 mt-4 text-xs">
              <div className="flex items-center gap-1">
                <div className="w-3 h-3 rounded bg-red-600"></div>
                <span>Hot (Above Expected)</span>
              </div>
              <div className="flex items-center gap-1">
                <div className="w-3 h-3 rounded bg-gray-700"></div>
                <span>Neutral</span>
              </div>
              <div className="flex items-center gap-1">
                <div className="w-3 h-3 rounded bg-blue-600"></div>
                <span>Cold (Below Expected)</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
