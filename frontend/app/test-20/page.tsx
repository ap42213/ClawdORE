'use client'

import { useState, useEffect, useRef } from 'react'

interface LogEntry {
  id: number
  timestamp: string
  bot: string
  type: 'info' | 'win' | 'loss' | 'decision' | 'action'
  message: string
}

const SQUARE_COUNT = 20

const BOT_COLORS: Record<string, string> = {
  'TEST-20': '#a855f7',
  'ORE': '#ff6b35',
  'RESULT': '#fbbf24',
  'SYSTEM': '#64748b',
}

const STORAGE_KEY = 'test20_data'

export default function Test20Page() {
  const [logs, setLogs] = useState<LogEntry[]>([])
  const [currentRound, setCurrentRound] = useState<number>(0)
  const [recommendedSquares, setRecommendedSquares] = useState<number[]>([])
  const [stats, setStats] = useState({ wins: 0, losses: 0, total: 0, winRate: '0' })
  const [lastWinner, setLastWinner] = useState<{ round_id: number, winning_square: number } | null>(null)
  const [connected, setConnected] = useState(false)
  const [terminalPaused, setTerminalPaused] = useState(false)
  const [initialized, setInitialized] = useState(false)
  const terminalRef = useRef<HTMLDivElement>(null)
  const logIdRef = useRef(0)
  const startedRef = useRef(false)
  // Track which squares were locked for which round
  const lockedSquaresRef = useRef<Map<number, number[]>>(new Map())
  const processedResultsRef = useRef<Set<number>>(new Set())
  // Refs for interval to access current state
  const currentRoundRef = useRef(0)
  const recommendedSquaresRef = useRef<number[]>([])

  // Keep refs in sync
  useEffect(() => { currentRoundRef.current = currentRound }, [currentRound])
  useEffect(() => { recommendedSquaresRef.current = recommendedSquares }, [recommendedSquares])

  // Load from localStorage on mount
  useEffect(() => {
    try {
      const saved = localStorage.getItem(STORAGE_KEY)
      if (saved) {
        const data = JSON.parse(saved)
        if (data.stats) setStats(data.stats)
        if (data.processedResults) {
          processedResultsRef.current = new Set(data.processedResults)
        }
        if (data.lockedSquares) {
          lockedSquaresRef.current = new Map(data.lockedSquares)
        }
        if (data.currentRound) setCurrentRound(data.currentRound)
        if (data.logs) {
          setLogs(data.logs)
          logIdRef.current = data.logs.length
        }
      }
    } catch (e) {
      console.error('Failed to load saved data:', e)
    }
    setInitialized(true)
  }, [])

  // Save to localStorage when stats change
  useEffect(() => {
    if (!initialized) return
    try {
      const data = {
        stats,
        processedResults: Array.from(processedResultsRef.current),
        lockedSquares: Array.from(lockedSquaresRef.current.entries()),
        currentRound,
        logs: logs.slice(-100), // Keep last 100 logs
      }
      localStorage.setItem(STORAGE_KEY, JSON.stringify(data))
    } catch (e) {
      console.error('Failed to save data:', e)
    }
  }, [stats, currentRound, logs, initialized])

  const addLog = (bot: string, type: LogEntry['type'], message: string) => {
    const entry: LogEntry = {
      id: logIdRef.current++,
      timestamp: new Date().toISOString(),
      bot,
      type,
      message,
    }
    setLogs(prev => [...prev.slice(-200), entry])
  }

  const resetStats = () => {
    setStats({ wins: 0, losses: 0, total: 0, winRate: '0' })
    processedResultsRef.current = new Set()
    lockedSquaresRef.current = new Map()
    setLogs([])
    logIdRef.current = 0
    localStorage.removeItem(STORAGE_KEY)
    addLog('SYSTEM', 'info', '🔄 Stats reset')
  }

  const formatTime = (iso: string) => {
    const d = new Date(iso)
    return d.toLocaleTimeString('en-US', { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit' })
  }

  // Startup message - only once
  useEffect(() => {
    if (!initialized) return
    if (startedRef.current) return
    startedRef.current = true
    
    if (stats.total > 0) {
      addLog('SYSTEM', 'info', `📊 Restored: ${stats.wins}W/${stats.losses}L (${stats.winRate}%)`)
    } else {
      addLog('SYSTEM', 'info', `🚀 20-Square Test Mode Started (${SQUARE_COUNT}/25 = 80% expected win rate)`)
    }
  }, [initialized])

  // Fetch live data
  useEffect(() => {
    if (!initialized) return
    
    const fetchData = async () => {
      try {
        // Fetch current state
        const stateRes = await fetch('/api/state')
        if (!stateRes.ok) return
        
        const state = await stateRes.json()
        
        // Calculate scores for all 25 squares from backend data
        const squareScores: Record<number, number> = {}
        for (let sq = 1; sq <= 25; sq++) {
          squareScores[sq] = 0
        }
        
        // 1. Add scores from current_strategies (backend strategies with weights)
        if (state.current_strategies && Array.isArray(state.current_strategies)) {
          for (const strategy of state.current_strategies) {
            const squares = strategy.squares || []
            const weights = strategy.weights || []
            const confidence = strategy.confidence || 0.5
            
            for (let i = 0; i < squares.length; i++) {
              const sq = squares[i]
              const weight = weights[i] || 0.1
              if (sq >= 1 && sq <= 25) {
                squareScores[sq] += weight * confidence
              }
            }
          }
        }
        
        // 2. Add scores from analytics_predictions (top squares)
        if (state.analytics_predictions?.top_squares) {
          const topSquares = state.analytics_predictions.top_squares
          const analyticsConfidence = state.analytics_predictions.confidence || 0.5
          for (let i = 0; i < topSquares.length; i++) {
            const sq = topSquares[i]
            if (sq >= 1 && sq <= 25) {
              squareScores[sq] += (topSquares.length - i) * analyticsConfidence * 0.1
            }
          }
        }
        
        // 3. Add scores from consensus_recommendation
        if (state.consensus_recommendation) {
          const recSquares = state.consensus_recommendation.squares || []
          const recWeights = state.consensus_recommendation.weights || []
          const recConfidence = state.consensus_recommendation.confidence || 0.5
          
          for (let i = 0; i < recSquares.length; i++) {
            const sq = recSquares[i]
            const weight = recWeights[i] || 0.3
            if (sq >= 1 && sq <= 25) {
              squareScores[sq] += weight * recConfidence * 2
            }
          }
        }
        
        // Sort squares by score (descending) and take best 20
        const sortedSquares = Object.entries(squareScores)
          .map(([sq, score]) => ({ square: Number(sq), score }))
          .sort((a, b) => b.score - a.score)
        
        const testSquares = sortedSquares.slice(0, SQUARE_COUNT).map(s => s.square).sort((a, b) => a - b)
        const excludedSquares = sortedSquares.slice(SQUARE_COUNT).map(s => s.square).sort((a, b) => a - b)
        
        // Update display squares (use ref to check current value)
        if (JSON.stringify(testSquares) !== JSON.stringify(recommendedSquaresRef.current)) {
          setRecommendedSquares(testSquares)
          addLog('TEST-20', 'decision', `🎯 Best ${SQUARE_COUNT} squares: [${testSquares.join(', ')}]`)
          addLog('TEST-20', 'info', `🚫 Excluded (worst 5): [${excludedSquares.join(', ')}]`)
        }
        
        // Track current round and lock squares for it (use ref to check current value)
        const roundId = Number(state.current_round) || 0
        if (roundId > 0 && roundId !== currentRoundRef.current) {
          // Lock current squares for this round (will be judged when round ends)
          lockedSquaresRef.current.set(roundId, [...testSquares])
          setCurrentRound(roundId)
          addLog('ORE', 'action', `🆕 Round #${roundId} started`)
          addLog('ORE', 'info', `🔒 Locked: [${testSquares.join(', ')}]`)
        }
        
        setConnected(true)

        // Fetch results and judge against LOCKED squares (not current)
        const resultsRes = await fetch('/api/results')
        if (!resultsRes.ok) return
        
        const data = await resultsRes.json()
        
        // Use last_winner for simplest check
        if (data.last_winner) {
          const rid = Number(data.last_winner.round_id)
          const winningSquare = Number(data.last_winner.winning_square)
          
          if (!processedResultsRef.current.has(rid) && winningSquare > 0) {
            // Check if we have locked squares for this round
            const lockedForRound = lockedSquaresRef.current.get(rid)
            
            if (lockedForRound && lockedForRound.length === SQUARE_COUNT) {
              processedResultsRef.current.add(rid)
              setLastWinner({ round_id: rid, winning_square: winningSquare })
              
              // Judge against the squares we had LOCKED for this round
              const hit = lockedForRound.includes(winningSquare)
              const excluded = [1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25].filter(s => !lockedForRound.includes(s))
              
              if (hit) {
                addLog('RESULT', 'win', `✅ ROUND #${rid} - Winner: □${winningSquare} - HIT!`)
                addLog('RESULT', 'info', `   Our picks: [${lockedForRound.join(', ')}]`)
                setStats(prev => ({
                  wins: prev.wins + 1,
                  losses: prev.losses,
                  total: prev.total + 1,
                  winRate: ((prev.wins + 1) / (prev.total + 1) * 100).toFixed(1)
                }))
              } else {
                addLog('RESULT', 'loss', `❌ ROUND #${rid} - Winner: □${winningSquare} - MISS!`)
                addLog('RESULT', 'info', `   We excluded: [${excluded.join(', ')}] - □${winningSquare} was in there!`)
                setStats(prev => ({
                  wins: prev.wins,
                  losses: prev.losses + 1,
                  total: prev.total + 1,
                  winRate: (prev.wins / (prev.total + 1) * 100).toFixed(1)
                }))
              }
              
              // Clean up old locked squares (keep last 10)
              if (lockedSquaresRef.current.size > 10) {
                const keys = Array.from(lockedSquaresRef.current.keys()).sort((a, b) => a - b)
                for (const key of keys.slice(0, keys.length - 10)) {
                  lockedSquaresRef.current.delete(key)
                }
              }
            } else {
              // No locked squares for this round - just show the winner
              if (!processedResultsRef.current.has(rid)) {
                addLog('ORE', 'info', `⏭️ Round #${rid} completed - Square ${winningSquare} won (not tracked - page wasn't open)`)
                processedResultsRef.current.add(rid)
              }
            }
          }
        }
      } catch (e) {
        setConnected(false)
        console.error('Fetch error:', e)
      }
    }
    
    fetchData()
    const interval = setInterval(fetchData, 3000)
    return () => clearInterval(interval)
  }, [initialized]) // Only depend on initialized, not on state that changes frequently

  // Auto-scroll terminal
  useEffect(() => {
    if (terminalRef.current && !terminalPaused) {
      terminalRef.current.scrollTop = terminalRef.current.scrollHeight
    }
  }, [logs, terminalPaused])

  const getTypeStyle = (type: LogEntry['type']) => {
    switch (type) {
      case 'win': return 'text-green-400 font-bold'
      case 'loss': return 'text-red-400 font-bold'
      case 'action': return 'text-blue-400'
      case 'decision': return 'text-yellow-400'
      default: return 'text-gray-400'
    }
  }

  return (
    <main className="min-h-screen bg-[#0a0a0f] text-white font-mono">
      {/* Header */}
      <header className="border-b border-gray-800 px-6 py-4">
        <div className="max-w-6xl mx-auto flex items-center justify-between">
          <div className="flex items-center gap-4">
            <h1 className="text-2xl font-bold">
              <span className="text-purple-500">🎯</span> 20-Square Test
            </h1>
            <span className="text-gray-500 text-sm">Testing {SQUARE_COUNT}/25 Coverage</span>
          </div>
          <div className="flex items-center gap-6 text-sm">
            <div className="flex items-center gap-2">
              <div className={`w-2 h-2 rounded-full ${connected ? 'bg-green-500 animate-pulse' : 'bg-red-500'}`} />
              <span className="text-gray-400">{connected ? 'Connected' : 'Offline'}</span>
            </div>
            <div className="text-gray-500">
              Round <span className="text-white font-bold">#{currentRound || '—'}</span>
            </div>
          </div>
        </div>
      </header>

      <div className="max-w-6xl mx-auto p-6 flex gap-6">
        {/* Sidebar */}
        <aside className="w-48 flex-shrink-0">
          {/* Stats */}
          <div className="p-3 bg-gray-900/50 rounded-lg border border-gray-800 mb-4">
            <h2 className="text-xs text-gray-500 uppercase tracking-wider mb-2">Win/Loss Stats</h2>
            <div className="space-y-2 text-sm">
              <div className="flex justify-between">
                <span className="text-gray-500">Win Rate</span>
                <span className="text-yellow-400 font-bold text-lg">
                  {stats.winRate}%
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-500">Record</span>
                <span>
                  <span className="text-green-400 font-bold">{stats.wins}W</span>
                  <span className="text-gray-600"> / </span>
                  <span className="text-red-400 font-bold">{stats.losses}L</span>
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-500">Total</span>
                <span className="text-white">{stats.total}</span>
              </div>
            </div>
          </div>

          {/* Coverage */}
          <div className="p-3 bg-purple-900/30 rounded-lg border border-purple-600/50 mb-4">
            <h2 className="text-xs text-gray-500 uppercase tracking-wider mb-2">Coverage</h2>
            <div className="text-center">
              <div className="text-3xl font-bold text-purple-400">{SQUARE_COUNT}/25</div>
              <div className="text-sm text-gray-400">80% Expected</div>
            </div>
          </div>

          {/* Current Squares */}
          <div className="p-3 bg-gray-900/50 rounded-lg border border-gray-800 mb-4">
            <h2 className="text-xs text-gray-500 uppercase tracking-wider mb-2">Testing Squares</h2>
            <div className="flex flex-wrap gap-1">
              {recommendedSquares.map(sq => (
                <span key={sq} className="bg-purple-600 px-1.5 py-0.5 rounded text-xs font-mono">
                  {sq}
                </span>
              ))}
            </div>
          </div>

          {/* Last Winner */}
          {lastWinner && (
            <div className="p-3 bg-orange-900/20 rounded-lg border border-orange-800/50 mb-4">
              <h2 className="text-xs text-gray-500 uppercase tracking-wider mb-2">Last Winner</h2>
              <div className="text-center">
                <div className="text-2xl font-bold" style={{ color: '#ff6b35' }}>
                  □ {lastWinner.winning_square}
                </div>
                <div className="text-xs text-gray-500 mt-1">
                  Round #{lastWinner.round_id}
                </div>
                <div className={`text-sm mt-1 font-bold ${
                  recommendedSquares.includes(lastWinner.winning_square) ? 'text-green-400' : 'text-red-400'
                }`}>
                  {recommendedSquares.includes(lastWinner.winning_square) ? '✅ HIT' : '❌ MISS'}
                </div>
              </div>
            </div>
          )}

          {/* Reset button */}
          <button 
            onClick={resetStats}
            className="w-full mb-4 px-3 py-2 bg-red-900/30 border border-red-800/50 rounded-lg text-red-400 text-xs hover:bg-red-900/50 transition-colors"
          >
            🔄 Reset Stats
          </button>

          {/* Back link */}
          <a href="/" className="block text-center text-purple-400 hover:underline text-sm">
            ← Back to Dashboard
          </a>
        </aside>

        {/* Terminal */}
        <div className="flex-1 bg-[#0d0d14] rounded-lg border border-gray-800 overflow-hidden">
          {/* Terminal Header */}
          <div className="flex items-center justify-between px-4 py-2 bg-[#12121a] border-b border-gray-800">
            <div className="flex items-center gap-2">
              <div className="w-3 h-3 rounded-full bg-red-500/80" />
              <div className="w-3 h-3 rounded-full bg-yellow-500/80" />
              <div className="w-3 h-3 rounded-full bg-green-500/80" />
            </div>
            <span className="text-xs text-gray-500">test-20 — live feed</span>
            <div className="flex items-center gap-4">
              <button
                onClick={() => setTerminalPaused(!terminalPaused)}
                className={`text-xs px-3 py-1 rounded transition-colors ${
                  terminalPaused 
                    ? 'bg-yellow-600 text-black font-bold' 
                    : 'bg-gray-700 text-gray-300 hover:bg-gray-600'
                }`}
              >
                {terminalPaused ? '▶ Resume' : '⏸ Pause'}
              </button>
              <div className="text-xs text-gray-600">{logs.length} events</div>
            </div>
          </div>

          {/* Terminal Content */}
          <div ref={terminalRef} className="h-[560px] overflow-y-auto p-4 text-sm leading-relaxed">
            {logs.length === 0 ? (
              <div className="text-gray-600 animate-pulse">
                Starting 20-square test...
              </div>
            ) : (
              logs.map(log => (
                <div key={log.id} className="flex gap-3 hover:bg-gray-900/30 py-0.5 px-1 -mx-1 rounded">
                  <span className="text-gray-600 flex-shrink-0 w-20">
                    {formatTime(log.timestamp)}
                  </span>
                  <span 
                    className="flex-shrink-0 w-24 truncate"
                    style={{ color: BOT_COLORS[log.bot] || '#888' }}
                  >
                    [{log.bot}]
                  </span>
                  <span className={getTypeStyle(log.type)}>
                    {log.message}
                  </span>
                </div>
              ))
            )}
            <div className="text-green-500 animate-pulse mt-2">▋</div>
          </div>
        </div>
      </div>

      {/* Footer */}
      <footer className="border-t border-gray-800 px-6 py-4 mt-4">
        <div className="max-w-6xl mx-auto flex items-center justify-between text-xs text-gray-600">
          <span>ClawdORE • 20-Square Coverage Test</span>
          <span>Expected: 80% Win Rate</span>
        </div>
      </footer>
    </main>
  )
}
