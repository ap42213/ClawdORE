'use client'

import { useState, useEffect, useCallback, useRef } from 'react'
import BotCard from './components/BotCard'
import Terminal from './components/Terminal'
import Stats from './components/Stats'
import BoardGrid from './components/BoardGrid'
import StrategyPanel from './components/StrategyPanel'
import RoundTimer from './components/RoundTimer'

// Local storage keys for persistence
const STORAGE_KEYS = {
  ROUND: 'clawdore_round',
  RECOMMENDATION: 'clawdore_recommendation',
  STATS: 'clawdore_stats',
  LOGS: 'clawdore_logs',
} as const

// Helper to safely get from localStorage
function getStoredValue<T>(key: string, fallback: T): T {
  if (typeof window === 'undefined') return fallback
  try {
    const stored = localStorage.getItem(key)
    return stored ? JSON.parse(stored) : fallback
  } catch {
    return fallback
  }
}

// Helper to safely set localStorage
function setStoredValue(key: string, value: any): void {
  if (typeof window === 'undefined') return
  try {
    localStorage.setItem(key, JSON.stringify(value))
  } catch {
    // Ignore storage errors
  }
}

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
  time_remaining_secs?: number
  round_duration_secs?: number
  updated_at?: string
}

export default function Home() {
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

  const [logs, setLogs] = useState<string[]>(() => 
    getStoredValue(STORAGE_KEYS.LOGS, [
      '🚀 ClawdORE Dashboard initialized',
      '📡 Connecting to database...',
    ])
  )

  const [stats, setStats] = useState(() => 
    getStoredValue(STORAGE_KEYS.STATS, {
      balance: '0.00',
      roundsWon: 0,
      totalDeployed: '0.00',
      activeBots: 0,
      currentRound: 0,
      playersTracked: 0,
      transactionsProcessed: 0,
    })
  )

  const [currentRound, setCurrentRound] = useState<RoundData | null>(() =>
    getStoredValue(STORAGE_KEYS.ROUND, null)
  )
  const [recommendation, setRecommendation] = useState<{
    squares: number[]
    weights: string[]
    confidence: number
  } | null>(() => getStoredValue(STORAGE_KEYS.RECOMMENDATION, null))

  const [strategies, setStrategies] = useState<{
    name: string
    squares: number[]
    weights: number[]
    reasoning: string
    confidence: number
    expected_roi: number
  }[]>([])

  const [signals, setSignals] = useState<Signal[]>([])
  const [mounted, setMounted] = useState(false)

  // Only run on client - restore from localStorage
  useEffect(() => {
    setMounted(true)
    // Restore persisted state on mount
    const storedRound = getStoredValue(STORAGE_KEYS.ROUND, null)
    const storedRec = getStoredValue(STORAGE_KEYS.RECOMMENDATION, null)
    const storedStats = getStoredValue(STORAGE_KEYS.STATS, null)
    const storedLogs = getStoredValue(STORAGE_KEYS.LOGS, null)
    
    if (storedRound) setCurrentRound(storedRound)
    if (storedRec) setRecommendation(storedRec)
    if (storedStats) setStats(storedStats)
    if (storedLogs && storedLogs.length > 2) setLogs(storedLogs)
  }, [])

  // Persist state changes to localStorage
  useEffect(() => {
    if (currentRound) setStoredValue(STORAGE_KEYS.ROUND, currentRound)
  }, [currentRound])

  useEffect(() => {
    if (recommendation) setStoredValue(STORAGE_KEYS.RECOMMENDATION, recommendation)
  }, [recommendation])

  useEffect(() => {
    setStoredValue(STORAGE_KEYS.STATS, stats)
  }, [stats])

  useEffect(() => {
    if (logs.length > 2) setStoredValue(STORAGE_KEYS.LOGS, logs.slice(-30))
  }, [logs])

  // Fetch data from API
  const fetchData = useCallback(async () => {
    if (typeof window === 'undefined') return
    
    try {
      // Fetch signals (heartbeats)
      const signalsRes = await fetch(`/api/signals?limit=50`, {
        cache: 'no-store',
        headers: { 'Content-Type': 'application/json' }
      })
      if (signalsRes.ok) {
        const data = await signalsRes.json()
        const signalsList = data.signals || []
        setSignals(signalsList)
        
        // Log for debugging
        if (signalsList.length > 0) {
          console.log('Signals received:', signalsList.length)
        }
        
        // Update bot statuses based on heartbeats
        const now = new Date()
        const heartbeats = signalsList.filter((s: Signal) => 
          s.signal_type.toLowerCase() === 'heartbeat'
        )
        
        console.log('Heartbeats found:', heartbeats.length)
        
        setBots(prev => prev.map(bot => {
          const lastHb = heartbeats.find((h: Signal) => {
            const src = h.source_bot.toLowerCase()
            return (
              src === bot.name.toLowerCase() || 
              src === bot.id.toLowerCase() || 
              src === (bot.id + '-bot').toLowerCase() ||
              (src === 'coordinator' && bot.id === 'coordinator')
            )
          })
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
          const sigType = sig.signal_type.toLowerCase()
          const emoji = sigType === 'heartbeat' ? '💓' : 
                       sigType === 'round_started' ? '🆕' : 
                       sigType === 'deploy_opportunity' ? '🎯' : '📨'
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
      const stateRes = await fetch(`/api/state`, { cache: 'no-store' })
      if (stateRes.ok) {
        const data = await stateRes.json()
        
        if (data.monitor_status) {
          // Parse deployed_squares - might be strings from database
          const deployedSquares = (data.monitor_status.deployed_squares || []).map((v: any) => 
            typeof v === 'string' ? parseInt(v, 10) : v
          )
          
          setCurrentRound({
            round_id: data.monitor_status.round_id,
            total_deployed: typeof data.monitor_status.total_deployed === 'string' 
              ? parseInt(data.monitor_status.total_deployed, 10) 
              : data.monitor_status.total_deployed,
            deployed_squares: deployedSquares,
            winning_square: data.monitor_status.winning_square,
            time_remaining_secs: data.monitor_status.time_remaining_secs,
            round_duration_secs: data.monitor_status.round_duration_secs || 60,
            updated_at: data.monitor_status.updated_at,
          })
          
          const totalDeployed = typeof data.monitor_status.total_deployed === 'string'
            ? parseInt(data.monitor_status.total_deployed, 10)
            : data.monitor_status.total_deployed
            
          setStats(prev => ({
            ...prev,
            currentRound: data.monitor_status.round_id,
            totalDeployed: (totalDeployed / 1e9).toFixed(4),
          }))
        }

        if (data.consensus_recommendation) {
          setRecommendation({
            squares: data.consensus_recommendation.squares,
            weights: data.consensus_recommendation.weights.map((w: number) => 
              (w * 100).toFixed(1) + '%'
            ),
            confidence: data.consensus_recommendation.confidence,
          })
        }

        // Set current strategies from database
        if (data.current_strategies && Array.isArray(data.current_strategies)) {
          setStrategies(data.current_strategies)
        }
      }

      // Fetch stats
      const statsRes = await fetch(`/api/stats`, { cache: 'no-store' })
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
      console.error('Fetch error:', error)
      setLogs(prev => [...prev.slice(-50), `❌ Error: ${error}`])
    }
  }, [])

  useEffect(() => {
    if (!mounted) return
    fetchData()
    const interval = setInterval(fetchData, 5000) // Poll every 5 seconds
    return () => clearInterval(interval)
  }, [mounted, fetchData])

  useEffect(() => {
    const active = bots.filter(bot => bot.status === 'online').length
    setStats(prev => ({ ...prev, activeBots: active }))
  }, [bots])

  return (
    <main className="dashboard">
      <div className="dashboard-container">
        {/* Header with Timer */}
        <header className="dashboard-header">
          <div className="header-content">
            <h1 className="dashboard-title">⛏️ ClawdORE</h1>
            <p className="dashboard-subtitle">ORE Mining Intelligence Network</p>
            <div className="header-status">
              <span className="status-online">● {stats.activeBots}/7 Bots Online</span>
              <span className="status-round">Round #{stats.currentRound}</span>
            </div>
          </div>
          
          {/* Round Timer */}
          {currentRound && (
            <div className="timer-wrapper">
              <RoundTimer 
                roundId={currentRound.round_id}
                timeRemaining={currentRound.time_remaining_secs}
                roundDuration={currentRound.round_duration_secs}
                updatedAt={currentRound.updated_at}
              />
            </div>
          )}
        </header>

        {/* Stats Row */}
        <Stats stats={stats} />

        {/* Main Grid */}
        <div className="main-grid">
          {/* Left: Board Grid */}
          <div className="grid-section">
            <BoardGrid 
              round={currentRound} 
              recommendation={recommendation}
            />
          </div>

          {/* Center: Strategy Panel */}
          <div className="grid-section">
            <StrategyPanel recommendation={recommendation} strategies={strategies} />
          </div>

          {/* Right: Terminal Feed */}
          <div className="grid-section">
            <Terminal logs={logs} signals={signals} />
          </div>
        </div>

        {/* Bot Cards Grid */}
        <section className="bots-section">
          <h2 className="section-title">🤖 Bot Network</h2>
          <div className="bots-grid">
            {bots.map(bot => (
              <BotCard key={bot.id} bot={bot} />
            ))}
          </div>
        </section>

        {/* Footer */}
        <footer className="dashboard-footer">
          <p>ClawdORE Intelligence Network • Powered by PostgreSQL</p>
        </footer>
      </div>
    </main>
  )
}
