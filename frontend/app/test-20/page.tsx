'use client'

import { useState, useEffect, useRef } from 'react'

interface LogEntry {
  id: number
  timestamp: string
  bot: string
  type: 'info' | 'win' | 'loss' | 'decision' | 'action'
  message: string
}

interface ServerStats {
  total: number
  hits: number
  misses: number
  win_rate: number
  baseline: number
  edge: number
  recent_results: Array<{
    round_id: number
    winning_square: number
    is_hit: boolean
    betting_squares: number[]
    skipping_squares: number[]
  }>
}

const SQUARE_COUNT = 20

const BOT_COLORS: Record<string, string> = {
  'TEST-20': '#a855f7',
  'ORE': '#ff6b35',
  'RESULT': '#fbbf24',
  'SYSTEM': '#64748b',
  'SERVER': '#22c55e',
}

export default function Test20Page() {
  const [logs, setLogs] = useState<LogEntry[]>([])
  const [currentRound, setCurrentRound] = useState<number>(0)
  const [bettingSquares, setBettingSquares] = useState<number[]>([])
  const [skippingSquares, setSkippingSquares] = useState<number[]>([])
  const [serverStats, setServerStats] = useState<ServerStats | null>(null)
  const [lastWinner, setLastWinner] = useState<{ round_id: number, winning_square: number, is_hit: boolean } | null>(null)
  const [connected, setConnected] = useState(false)
  const [terminalPaused, setTerminalPaused] = useState(false)
  const terminalRef = useRef<HTMLDivElement>(null)
  const logIdRef = useRef(0)
  const lastRoundRef = useRef(0)
  const lastResultRef = useRef(0)

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

  // Fetch server stats (runs even when laptop closed - from DB)
  useEffect(() => {
    const fetchServerStats = async () => {
      try {
        const res = await fetch('/api/test-20')
        if (!res.ok) return
        const data: ServerStats = await res.json()
        setServerStats(data)
        
        // Show new results in log
        if (data.recent_results && data.recent_results.length > 0) {
          const latest = data.recent_results[0]
          if (latest.round_id > lastResultRef.current) {
            lastResultRef.current = latest.round_id
            setLastWinner({
              round_id: latest.round_id,
              winning_square: latest.winning_square,
              is_hit: latest.is_hit
            })
            
            if (latest.is_hit) {
              addLog('SERVER', 'win', `✅ #${latest.round_id}: □${latest.winning_square} won - HIT!`)
            } else {
              addLog('SERVER', 'loss', `❌ #${latest.round_id}: □${latest.winning_square} won - MISS! (skipped: ${latest.skipping_squares.join(',')})`)
            }
          }
        }
        setConnected(true)
      } catch (e) {
        console.error('Failed to fetch server stats:', e)
      }
    }

    fetchServerStats()
    const interval = setInterval(fetchServerStats, 5000)
    return () => clearInterval(interval)
  }, [])

  // Fetch current round state for live display
  useEffect(() => {
    const fetchState = async () => {
      try {
        const res = await fetch('/api/state')
        if (!res.ok) return
        const state = await res.json()
        
        const roundId = Number(state.current_round) || 0
        if (roundId > 0 && roundId !== lastRoundRef.current) {
          lastRoundRef.current = roundId
          setCurrentRound(roundId)
          addLog('ORE', 'action', `🆕 Round #${roundId} started`)
        }
        
        // Get current betting/skipping from consensus
        if (state.consensus_recommendation?.squares) {
          const consensus = state.consensus_recommendation.squares as number[]
          // Server calculates best 20 - for display, show what server picked
        }
      } catch (e) {
        console.error('Failed to fetch state:', e)
      }
    }

    fetchState()
    const interval = setInterval(fetchState, 3000)
    return () => clearInterval(interval)
  }, [])

  // Initial log
  useEffect(() => {
    addLog('SYSTEM', 'info', '🚀 Test-20 Tracker - Server-side tracking (runs 24/7)')
    addLog('SYSTEM', 'info', '📊 Stats from database - persists even when browser closed')
  }, [])

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

  const stats = serverStats || { total: 0, hits: 0, misses: 0, win_rate: 0, baseline: 80, edge: 0 }
  const edgeColor = stats.edge > 0 ? 'text-green-400' : stats.edge < 0 ? 'text-red-400' : 'text-gray-400'

  return (
    <main className="min-h-screen bg-[#0a0a0f] text-white font-mono">
      {/* Header */}
      <header className="border-b border-gray-800 px-6 py-4">
        <div className="max-w-6xl mx-auto flex items-center justify-between">
          <div className="flex items-center gap-4">
            <h1 className="text-2xl font-bold">
              <span className="text-purple-500">🎯</span> 20-Square Test
            </h1>
            <span className="text-gray-500 text-sm">Server-side tracking (24/7)</span>
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
        <aside className="w-56 flex-shrink-0">
          {/* Server Stats */}
          <div className="p-4 bg-gray-900/50 rounded-lg border border-gray-800 mb-4">
            <h2 className="text-xs text-gray-500 uppercase tracking-wider mb-3">📊 Server Stats (24/7)</h2>
            <div className="space-y-3 text-sm">
              <div className="flex justify-between items-center">
                <span className="text-gray-500">Win Rate</span>
                <span className="text-yellow-400 font-bold text-2xl">
                  {stats.win_rate.toFixed(1)}%
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-500">Record</span>
                <span>
                  <span className="text-green-400 font-bold">{stats.hits}W</span>
                  <span className="text-gray-600"> / </span>
                  <span className="text-red-400 font-bold">{stats.misses}L</span>
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-500">Total Rounds</span>
                <span className="text-white">{stats.total}</span>
              </div>
              <div className="border-t border-gray-700 pt-2 mt-2">
                <div className="flex justify-between">
                  <span className="text-gray-500">Baseline</span>
                  <span className="text-gray-400">{stats.baseline}%</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-500">Edge</span>
                  <span className={`font-bold ${edgeColor}`}>
                    {stats.edge > 0 ? '+' : ''}{stats.edge.toFixed(1)}%
                  </span>
                </div>
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

          {/* Current Betting Squares */}
          {serverStats?.recent_results?.[0] && (
            <div className="p-3 bg-green-900/20 rounded-lg border border-green-800/50 mb-4">
              <h2 className="text-xs text-gray-500 uppercase tracking-wider mb-2">🎯 Betting On (20)</h2>
              <div className="flex flex-wrap gap-1">
                {[...serverStats.recent_results[0].betting_squares].sort((a,b) => a-b).map(sq => (
                  <span key={sq} className="bg-green-700 px-1.5 py-0.5 rounded text-xs font-mono">
                    {sq}
                  </span>
                ))}
              </div>
            </div>
          )}

          {/* Skipping Squares */}
          {serverStats?.recent_results?.[0] && (
            <div className="p-3 bg-red-900/20 rounded-lg border border-red-800/50 mb-4">
              <h2 className="text-xs text-gray-500 uppercase tracking-wider mb-2">🚫 Skipping (5)</h2>
              <div className="flex flex-wrap gap-1">
                {[...serverStats.recent_results[0].skipping_squares].sort((a,b) => a-b).map(sq => (
                  <span key={sq} className="bg-red-700 px-1.5 py-0.5 rounded text-xs font-mono">
                    {sq}
                  </span>
                ))}
              </div>
            </div>
          )}

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
                <div className={`text-sm mt-1 font-bold ${lastWinner.is_hit ? 'text-green-400' : 'text-red-400'}`}>
                  {lastWinner.is_hit ? '✅ HIT' : '❌ MISS'}
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
                Connecting to server...
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
          <span>ClawdORE • Server-side Test-20 Tracker</span>
          <span>Expected: 80% • Tracking 24/7 on Railway</span>
        </div>
      </footer>
    </main>
  )
}
