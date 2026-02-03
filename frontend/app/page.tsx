'use client'

import { useState, useEffect, useCallback } from 'react'
import BotCard from './components/BotCard'
import Terminal from './components/Terminal'
import Stats from './components/Stats'
import BoardGrid from './components/BoardGrid'
import StrategyPanel from './components/StrategyPanel'

export interface Bot {
  id: string
  name: string
  displayName: string
  status: 'online' | 'offline' | 'syncing'
  description: string
  icon: string
  lastHeartbeat?: string
  metrics?: {
    label: string
    value: string
  }[]
}

export interface Signal {
  id: number
  signal_type: string
  source_bot: string
  payload: any
  created_at: string
}

export interface RoundData {
  round_id: number
  total_deployed: number
  deployed_squares: number[]
  winning_square?: number
}

export default function Home() {
  const API_URL = process.env.NEXT_PUBLIC_API_URL || ''

  const [bots, setBots] = useState<Bot[]>([
    {
      id: 'coordinator',
      name: 'coordinator-bot',
      displayName: 'CLAWDOREDINATOR',
      status: 'offline',
      description: 'Central hub - coordinates all bots and learning',
      icon: '🧠',
    },
    {
      id: 'monitor',
      name: 'monitor-bot',
      displayName: 'MONITORE',
      status: 'offline',
      description: 'Real-time board and treasury monitoring',
      icon: '👁️',
    },
    {
      id: 'analytics',
      name: 'analytics-bot',
      displayName: 'ANALYTICORE',
      status: 'offline',
      description: 'Pattern analysis and predictions',
      icon: '📊',
    },
    {
      id: 'parser',
      name: 'parser-bot',
      displayName: 'PARSEORE',
      status: 'offline',
      description: 'Transaction parsing and storage',
      icon: '🔍',
    },
    {
      id: 'learning',
      name: 'learning-bot',
      displayName: 'LEARNORE',
      status: 'offline',
      description: 'Deep wallet pattern learning',
      icon: '🎓',
    },
    {
      id: 'betting',
      name: 'betting-bot',
      displayName: 'BETORE',
      status: 'offline',
      description: 'Strategy execution engine',
      icon: '🎲',
    },
    {
      id: 'miner',
      name: 'miner-bot',
      displayName: 'MINEORE',
      status: 'offline',
      description: 'Executes trades based on consensus',
      icon: '⛏️',
    },
  ])

  const [logs, setLogs] = useState<string[]>([
    '🚀 ClawdORE Dashboard initialized',
    '📡 Connecting to database...',
  ])

  const [stats, setStats] = useState({
    balance: '0.00',
    roundsWon: 0,
    totalDeployed: '0.00',
    activeBots: 0,
    currentRound: 0,
    playersTracked: 0,
    transactionsProcessed: 0,
  })

  const [currentRound, setCurrentRound] = useState<RoundData | null>(null)
  const [recommendation, setRecommendation] = useState<{
    squares: number[]
    weights: string[]
    confidence: number
  } | null>(null)

  const [signals, setSignals] = useState<Signal[]>([])

  // Fetch data from API
  const fetchData = useCallback(async () => {
    try {
      // Fetch signals (heartbeats)
      const signalsRes = await fetch(`${API_URL}/api/signals?limit=50`)
      if (signalsRes.ok) {
        const data = await signalsRes.json()
        setSignals(data.signals || [])
        
        // Update bot statuses based on heartbeats
        const now = new Date()
        const heartbeats = (data.signals || []).filter((s: Signal) => s.signal_type === 'Heartbeat')
        
        setBots(prev => prev.map(bot => {
          const lastHb = heartbeats.find((h: Signal) => 
            h.source_bot === bot.name || h.source_bot === bot.id + '-bot'
          )
          if (lastHb) {
            const hbTime = new Date(lastHb.created_at)
            const diffSecs = (now.getTime() - hbTime.getTime()) / 1000
            return {
              ...bot,
              status: diffSecs < 60 ? 'online' : diffSecs < 300 ? 'syncing' : 'offline',
              lastHeartbeat: lastHb.created_at,
            }
          }
          return bot
        }))

        // Add new signals to logs
        const newSignals = (data.signals || []).slice(0, 5)
        newSignals.forEach((sig: Signal) => {
          const emoji = sig.signal_type === 'Heartbeat' ? '💓' : 
                       sig.signal_type === 'RoundStarted' ? '🆕' : '📨'
          const logMsg = `${emoji} [${sig.source_bot}] ${sig.signal_type}`
          setLogs(prev => {
            if (!prev.includes(logMsg)) {
              return [...prev.slice(-50), logMsg]
            }
            return prev
          })
        })
      }

      // Fetch current state
      const stateRes = await fetch(`${API_URL}/api/state`)
      if (stateRes.ok) {
        const data = await stateRes.json()
        
        if (data.monitor_status) {
          setCurrentRound({
            round_id: data.monitor_status.round_id,
            total_deployed: data.monitor_status.total_deployed,
            deployed_squares: data.monitor_status.deployed_squares || [],
            winning_square: data.monitor_status.winning_square,
          })
          setStats(prev => ({
            ...prev,
            currentRound: data.monitor_status.round_id,
            totalDeployed: (data.monitor_status.total_deployed / 1e9).toFixed(4),
          }))
        }

        if (data.consensus_recommendation) {
          setRecommendation({
            squares: data.consensus_recommendation.squares,
            weights: data.consensus_recommendation.weights,
            confidence: data.consensus_recommendation.confidence,
          })
        }
      }

      // Fetch stats
      const statsRes = await fetch(`${API_URL}/api/stats`)
      if (statsRes.ok) {
        const data = await statsRes.json()
        setStats(prev => ({
          ...prev,
          playersTracked: data.players_tracked || 0,
          transactionsProcessed: data.transactions_count || 0,
          roundsWon: data.wins_tracked || 0,
        }))
      }

    } catch (error) {
      // Silent fail - API might not be ready yet
    }
  }, [API_URL])

  useEffect(() => {
    fetchData()
    const interval = setInterval(fetchData, 5000) // Poll every 5 seconds
    return () => clearInterval(interval)
  }, [fetchData])

  useEffect(() => {
    const active = bots.filter(bot => bot.status === 'online').length
    setStats(prev => ({ ...prev, activeBots: active }))
  }, [bots])

  return (
    <main className="min-h-screen p-4 md:p-8 bg-gradient-to-br from-slate-950 via-slate-900 to-slate-950">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="mb-8 text-center">
          <h1 className="text-5xl font-bold mb-2 bg-gradient-to-r from-orange-500 via-yellow-500 to-orange-600 bg-clip-text text-transparent">
            ⛏️ ClawdORE
          </h1>
          <p className="text-gray-400 text-lg">
            ORE Mining Intelligence Network
          </p>
          <div className="mt-2 flex justify-center gap-4 text-sm">
            <span className="text-green-400">● {stats.activeBots}/7 Bots Online</span>
            <span className="text-orange-400">Round #{stats.currentRound}</span>
          </div>
        </div>

        {/* Stats Row */}
        <Stats stats={stats} />

        {/* Main Grid */}
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-6 mb-8">
          {/* Left: Board Grid */}
          <div className="lg:col-span-1">
            <BoardGrid 
              round={currentRound} 
              recommendation={recommendation}
            />
          </div>

          {/* Center: Strategy Panel */}
          <div className="lg:col-span-1">
            <StrategyPanel recommendation={recommendation} />
          </div>

          {/* Right: Terminal Feed */}
          <div className="lg:col-span-1">
            <Terminal logs={logs} signals={signals} />
          </div>
        </div>

        {/* Bot Cards Grid */}
        <h2 className="text-2xl font-bold mb-4 text-gray-200">🤖 Bot Network</h2>
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 xl:grid-cols-7 gap-4 mb-8">
          {bots.map(bot => (
            <BotCard key={bot.id} bot={bot} />
          ))}
        </div>

        {/* Footer */}
        <div className="text-center text-gray-500 text-sm mt-8">
          <p>ClawdORE Intelligence Network • Powered by PostgreSQL</p>
        </div>
      </div>
    </main>
  )
}
