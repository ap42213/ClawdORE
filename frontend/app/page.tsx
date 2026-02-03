'use client'

import { useState, useEffect, useRef } from 'react'

interface LogEntry {
  id: number
  timestamp: string
  bot: string
  type: 'info' | 'decision' | 'action' | 'win' | 'error' | 'ai'
  message: string
}

interface BotStatus {
  name: string
  status: 'online' | 'offline' | 'thinking'
  lastSeen: string
}

const BOT_COLORS: Record<string, string> = {
  'CLAWDOREDINATOR': '#a855f7',
  'MINEORE': '#3b82f6',
  'MONITORE': '#06b6d4',
  'ANALYTICORE': '#22c55e',
  'PARSEORE': '#eab308',
  'LEARNORE': '#ec4899',
  'BETORE': '#f97316',
  'AI-ADVISORE': '#00d4aa',
  'SYSTEM': '#64748b',
  'ORE': '#ff6b35',
}

export default function Home() {
  const [logs, setLogs] = useState<LogEntry[]>([])
  const [bots, setBots] = useState<BotStatus[]>([
    { name: 'CLAWDOREDINATOR', status: 'online', lastSeen: '' },
    { name: 'MINEORE', status: 'online', lastSeen: '' },
    { name: 'MONITORE', status: 'online', lastSeen: '' },
    { name: 'ANALYTICORE', status: 'online', lastSeen: '' },
    { name: 'PARSEORE', status: 'online', lastSeen: '' },
    { name: 'LEARNORE', status: 'online', lastSeen: '' },
    { name: 'BETORE', status: 'online', lastSeen: '' },
  ])
  const [currentRound, setCurrentRound] = useState<number>(0)
  const [connected, setConnected] = useState(false)
  const [stats, setStats] = useState({ wins: 0, rounds: 0, oreEarned: 0 })
  const terminalRef = useRef<HTMLDivElement>(null)
  const logIdRef = useRef(0)

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

  // Fetch live data from API
  useEffect(() => {
    const fetchData = async () => {
      try {
        const signalsRes = await fetch('/api/signals?limit=50')
        if (signalsRes.ok) {
          const data = await signalsRes.json()
          const signals = data.signals || []
          
          const now = new Date()
          const heartbeats = signals.filter((s: any) => s.signal_type === 'Heartbeat')
          
          setBots(prev => prev.map(bot => {
            const hb = heartbeats.find((h: any) => {
              const src = h.source_bot.toUpperCase().replace('-BOT', '').replace('_BOT', '')
              const name = bot.name.replace('ORE', '')
              return src.includes(name) || name.includes(src) || 
                     (src === 'COORDINATOR' && bot.name === 'CLAWDOREDINATOR')
            })
            if (hb) {
              const hbTime = new Date(hb.created_at)
              const diffSecs = (now.getTime() - hbTime.getTime()) / 1000
              return {
                ...bot,
                status: diffSecs < 60 ? 'online' : diffSecs < 300 ? 'thinking' : 'offline',
                lastSeen: hb.created_at,
              }
            }
            return bot
          }))

          signals.slice(0, 15).forEach((sig: any) => {
            const botName = sig.source_bot.toUpperCase().replace('-BOT', '').replace('_BOT', '')
            const mappedBot = Object.keys(BOT_COLORS).find(b => 
              b.includes(botName) || botName.includes(b.replace('ORE', ''))
            ) || 'SYSTEM'
            
            let type: LogEntry['type'] = 'info'
            let message = sig.signal_type
            
            if (sig.signal_type === 'Heartbeat') {
              type = 'info'
              message = '♥ heartbeat'
            } else if (sig.signal_type === 'WinDetected') {
              type = 'win'
              message = `🏆 WIN DETECTED: Square ${sig.payload?.winning_square || '?'}`
            } else if (sig.signal_type === 'DeployDetected') {
              type = 'action'
              message = `📤 Deploy: ${sig.payload?.squares?.length || '?'} squares`
            } else if (sig.signal_type === 'RoundStarted') {
              type = 'decision'
              message = `🆕 Round ${sig.payload?.round_id || '?'} started`
            }

            setLogs(prev => {
              if (prev.some(l => l.timestamp === sig.created_at && l.bot === mappedBot)) {
                return prev
              }
              return [...prev.slice(-200), {
                id: logIdRef.current++,
                timestamp: sig.created_at,
                bot: mappedBot,
                type,
                message,
              }]
            })
          })
          
          setConnected(true)
        }

        const stateRes = await fetch('/api/state')
        if (stateRes.ok) {
          const data = await stateRes.json()
          if (data.monitor_status?.round_id) {
            setCurrentRound(data.monitor_status.round_id)
            
            // Log real ORE data
            const status = data.monitor_status
            if (status.time_remaining_secs !== undefined) {
              addLog('ORE', 'info', 
                `⏱️ Round #${status.round_id} - ${Math.round(status.time_remaining_secs)}s remaining, ${status.active_squares || 0} squares active`)
            }
          }
          
          // Log last deploy if available
          if (data.last_deploy) {
            const dep = data.last_deploy
            addLog('CLAWDOREDINATOR', 'action', 
              `⚡ Deployed squares [${dep.squares?.join(', ')}] - ${(dep.amount_lamports / 1e9).toFixed(4)} SOL`)
          }
          
          if (data.consensus_recommendation) {
            const rec = data.consensus_recommendation
            addLog('CLAWDOREDINATOR', 'decision', 
              `🎯 Consensus: squares [${rec.squares?.join(', ')}] (${Math.round((rec.confidence || 0) * 100)}% confidence)`)
          }
        }
        
        // Fetch real ORE blockchain data
        const oreRes = await fetch('/api/ore')
        if (oreRes.ok) {
          const oreData = await oreRes.json()
          if (oreData.round_id && oreData.round_id !== currentRound) {
            setCurrentRound(oreData.round_id)
            addLog('ORE', 'info', 
              `🆕 Round #${oreData.round_id} - ${oreData.total_deployed_sol?.toFixed(2) || 0} SOL total deployed`)
          }
        }

        // Fetch stats from database
        const statsRes = await fetch('/api/stats')
        if (statsRes.ok) {
          const statsData = await statsRes.json()
          setStats({
            wins: statsData.wins || 0,
            rounds: statsData.rounds || 0,
            oreEarned: statsData.ore_earned || 0,
          })
        }
      } catch (e) {
        setConnected(false)
      }
    }

    fetchData()
    const interval = setInterval(fetchData, 3000)
    return () => clearInterval(interval)
  }, [])

  // Auto-scroll terminal
  useEffect(() => {
    if (terminalRef.current) {
      terminalRef.current.scrollTop = terminalRef.current.scrollHeight
    }
  }, [logs])

  // NO DEMO - only real data from API/database
  // The bots write real ORE blockchain data to the database
  // This frontend just displays what's actually happening

  const getTypeStyle = (type: LogEntry['type']) => {
    switch (type) {
      case 'win': return 'text-green-400 font-bold'
      case 'error': return 'text-red-400'
      case 'action': return 'text-blue-400'
      case 'decision': return 'text-yellow-400'
      case 'ai': return 'text-cyan-400'
      default: return 'text-gray-400'
    }
  }

  const onlineBots = bots.filter(b => b.status === 'online').length

  return (
    <main className="min-h-screen bg-[#0a0a0f] text-white font-mono">
      {/* Header */}
      <header className="border-b border-gray-800 px-6 py-4">
        <div className="max-w-6xl mx-auto flex items-center justify-between">
          <div className="flex items-center gap-4">
            <h1 className="text-2xl font-bold">
              <span className="text-orange-500">⛏️</span> ClawdORE
            </h1>
            <span className="text-gray-500 text-sm">Live Bot Activity</span>
          </div>
          <div className="flex items-center gap-6 text-sm">
            <div className="flex items-center gap-2">
              <div className={`w-2 h-2 rounded-full ${connected ? 'bg-green-500 animate-pulse' : 'bg-red-500'}`} />
              <span className="text-gray-400">{connected ? 'Connected' : 'Offline Mode'}</span>
            </div>
            <div className="text-gray-500">
              Round <span className="text-white font-bold">#{currentRound || '—'}</span>
            </div>
            <div className="text-gray-500">
              <span className="text-green-400">{onlineBots}</span>/7 bots
            </div>
          </div>
        </div>
      </header>

      <div className="max-w-6xl mx-auto p-6 flex gap-6">
        {/* Bot Status Sidebar */}
        <aside className="w-44 flex-shrink-0">
          <h2 className="text-xs text-gray-500 uppercase tracking-wider mb-3">Bot Swarm</h2>
          <div className="space-y-2">
            {bots.map(bot => (
              <div key={bot.name} className="flex items-center gap-2 text-sm">
                <div className={`w-2 h-2 rounded-full ${
                  bot.status === 'online' ? 'bg-green-500' :
                  bot.status === 'thinking' ? 'bg-yellow-500 animate-pulse' :
                  'bg-gray-600'
                }`} />
                <span style={{ color: BOT_COLORS[bot.name] || '#fff' }} className="text-xs truncate">
                  {bot.name}
                </span>
              </div>
            ))}
          </div>

          {/* Stats */}
          <div className="mt-6 p-3 bg-gray-900/50 rounded-lg border border-gray-800">
            <h2 className="text-xs text-gray-500 uppercase tracking-wider mb-2">Performance</h2>
            <div className="space-y-2 text-sm">
              <div className="flex justify-between">
                <span className="text-gray-500">Win Rate</span>
                <span className="text-green-400 font-bold">
                  {stats.rounds > 0 ? ((stats.wins / stats.rounds) * 100).toFixed(1) : 0}%
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-500">Wins</span>
                <span className="text-white">{stats.wins}/{stats.rounds}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-500">ORE Earned</span>
                <span style={{ color: '#ff6b35' }}>{stats.oreEarned.toFixed(2)}</span>
              </div>
            </div>
          </div>
          
          <div className="mt-6">
            <h2 className="text-xs text-gray-500 uppercase tracking-wider mb-3">Legend</h2>
            <div className="space-y-1 text-xs">
              <div className="text-yellow-400">🎯 Decision</div>
              <div className="text-blue-400">📤 Action</div>
              <div className="text-green-400">🏆 Win</div>
              <div className="text-cyan-400">🤖 AI Insight</div>
              <div style={{ color: '#ff6b35' }}>⛏️ ORE Chain</div>
            </div>
          </div>
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
            <span className="text-xs text-gray-500">clawdore-swarm — live feed</span>
            <div className="text-xs text-gray-600">{logs.length} events</div>
          </div>

          {/* Terminal Content */}
          <div ref={terminalRef} className="h-[560px] overflow-y-auto p-4 text-sm leading-relaxed">
            {logs.length === 0 ? (
              <div className="text-gray-600 animate-pulse">
                Connecting to bot swarm...
              </div>
            ) : (
              logs.map(log => (
                <div key={log.id} className="flex gap-3 hover:bg-gray-900/30 py-0.5 px-1 -mx-1 rounded">
                  <span className="text-gray-600 flex-shrink-0 w-20">
                    {formatTime(log.timestamp)}
                  </span>
                  <span 
                    className="flex-shrink-0 w-36 truncate"
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
          <span>ClawdORE • 7-Bot Autonomous Mining Swarm</span>
          <span>Built with AI for Solana</span>
        </div>
      </footer>
    </main>
  )
}
