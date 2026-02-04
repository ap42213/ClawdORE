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

export default function Test20Page() {
  const [logs, setLogs] = useState<LogEntry[]>([])
  const [currentRound, setCurrentRound] = useState<number>(0)
  const [recommendedSquares, setRecommendedSquares] = useState<number[]>([])
  const [stats, setStats] = useState({ wins: 0, losses: 0, total: 0, winRate: '0' })
  const [lastWinner, setLastWinner] = useState<{ round_id: number, winning_square: number } | null>(null)
  const [connected, setConnected] = useState(false)
  const [terminalPaused, setTerminalPaused] = useState(false)
  const terminalRef = useRef<HTMLDivElement>(null)
  const logIdRef = useRef(0)
  const seenRoundsRef = useRef<Set<number>>(new Set())

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

  const formatTime = (iso: string) => {
    const d = new Date(iso)
    return d.toLocaleTimeString('en-US', { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit' })
  }

  // Fetch live data
  useEffect(() => {
    const fetchData = async () => {
      try {
        // Fetch current state
        const stateRes = await fetch('/api/state')
        if (stateRes.ok) {
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
                // Higher score for higher ranked squares
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
                squareScores[sq] += weight * recConfidence * 2 // Consensus gets extra weight
              }
            }
          }
          
          // Sort squares by score (descending) and take best 20
          const sortedSquares = Object.entries(squareScores)
            .map(([sq, score]) => ({ square: Number(sq), score }))
            .sort((a, b) => b.score - a.score)
          
          const testSquares = sortedSquares.slice(0, SQUARE_COUNT).map(s => s.square).sort((a, b) => a - b)
          const excludedSquares = sortedSquares.slice(SQUARE_COUNT).map(s => s.square).sort((a, b) => a - b)
          
          if (JSON.stringify(testSquares) !== JSON.stringify(recommendedSquares)) {
            setRecommendedSquares(testSquares)
            addLog('TEST-20', 'decision', `🎯 Best ${SQUARE_COUNT} squares: [${testSquares.join(', ')}]`)
            addLog('TEST-20', 'info', `🚫 Excluded (worst 5): [${excludedSquares.join(', ')}]`)
          }
          
          if (state.current_round) {
            const roundId = Number(state.current_round) || 0
            if (roundId !== currentRound && roundId > 0) {
              setCurrentRound(roundId)
              addLog('ORE', 'info', `🆕 Round #${roundId} started`)
            }
          }
          
          setConnected(true)
        }

        // Fetch results
        const resultsRes = await fetch('/api/results')
        if (resultsRes.ok) {
          const data = await resultsRes.json()
          const results = data.results || []
          
          for (const result of results.slice(0, 20)) {
            if (!seenRoundsRef.current.has(result.round_id)) {
              seenRoundsRef.current.add(result.round_id)
              
              const winningSquare = result.winning_square
              setLastWinner({ round_id: result.round_id, winning_square })
              
              // Check if our 20 squares would have hit
              const hit = recommendedSquares.includes(winningSquare)
              
              if (hit) {
                addLog('RESULT', 'win', `✅ ROUND #${result.round_id} - Square ${winningSquare} - HIT! (${SQUARE_COUNT}/25 coverage)`)
                setStats(prev => ({
                  wins: prev.wins + 1,
                  losses: prev.losses,
                  total: prev.total + 1,
                  winRate: ((prev.wins + 1) / (prev.total + 1) * 100).toFixed(1)
                }))
              } else {
                addLog('RESULT', 'loss', `❌ ROUND #${result.round_id} - Square ${winningSquare} - MISS (not in our ${SQUARE_COUNT})`)
                setStats(prev => ({
                  wins: prev.wins,
                  losses: prev.losses + 1,
                  total: prev.total + 1,
                  winRate: (prev.wins / (prev.total + 1) * 100).toFixed(1)
                }))
              }
            }
          }
        }
      } catch (e) {
        setConnected(false)
        console.error('Fetch error:', e)
      }
    }

    addLog('SYSTEM', 'info', `🚀 20-Square Test Mode Started (${SQUARE_COUNT}/25 = 80% expected win rate)`)
    
    fetchData()
    const interval = setInterval(fetchData, 3000)
    return () => clearInterval(interval)
  }, [recommendedSquares, currentRound])

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
