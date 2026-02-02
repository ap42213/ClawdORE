'use client'

import { useState, useEffect } from 'react'
import BotCard from './components/BotCard'
import Terminal from './components/Terminal'
import Stats from './components/Stats'

export interface Bot {
  id: string
  name: string
  status: 'running' | 'stopped' | 'starting'
  description: string
  icon: string
}

export default function Home() {
  const [bots, setBots] = useState<Bot[]>([
    {
      id: 'monitor',
      name: 'Monitor Bot',
      status: 'stopped',
      description: 'Monitors wallet balance and round status',
      icon: '👁️',
    },
    {
      id: 'analytics',
      name: 'Analytics Bot',
      status: 'stopped',
      description: 'Analyzes past rounds and predicts outcomes',
      icon: '📊',
    },
    {
      id: 'miner',
      name: 'Miner Bot',
      status: 'stopped',
      description: 'Mines ORE tokens automatically',
      icon: '⛏️',
    },
    {
      id: 'betting',
      name: 'Betting Bot',
      status: 'stopped',
      description: 'Places strategic bets on squares',
      icon: '🎲',
    },
  ])

  const [logs, setLogs] = useState<string[]>([
    '🚀 ClawdBot Dashboard initialized',
    '📡 Connecting to backend...',
  ])

  const [stats, setStats] = useState({
    balance: '0.00',
    roundsWon: 0,
    totalMined: '0.00',
    activeBots: 0,
  })

  useEffect(() => {
    // Update active bots count
    const active = bots.filter(bot => bot.status === 'running').length
    setStats(prev => ({ ...prev, activeBots: active }))
  }, [bots])

  const handleBotAction = async (botId: string, action: 'start' | 'stop') => {
    // Update bot status
    setBots(prev =>
      prev.map(bot =>
        bot.id === botId
          ? { ...bot, status: action === 'start' ? 'starting' : 'stopped' }
          : bot
      )
    )

    // Add log
    const bot = bots.find(b => b.id === botId)
    setLogs(prev => [...prev, `${action === 'start' ? '▶️' : '⏹️'} ${bot?.name} ${action}ing...`])

    // Simulate API call to Railway backend
    try {
      const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3000'
      const response = await fetch(`${API_URL}/api/bots/${botId}/${action}`, {
        method: 'POST',
      })

      if (response.ok) {
        // Update to running after successful start
        if (action === 'start') {
          setTimeout(() => {
            setBots(prev =>
              prev.map(bot =>
                bot.id === botId ? { ...bot, status: 'running' } : bot
              )
            )
            setLogs(prev => [...prev, `✅ ${bot?.name} started successfully`])
          }, 1500)
        } else {
          setLogs(prev => [...prev, `✅ ${bot?.name} stopped successfully`])
        }
      } else {
        throw new Error('Failed to ' + action + ' bot')
      }
    } catch (error) {
      setLogs(prev => [...prev, `❌ Error: ${error}`])
      // Revert status on error
      setBots(prev =>
        prev.map(bot =>
          bot.id === botId ? { ...bot, status: 'stopped' } : bot
        )
      )
    }
  }

  return (
    <main className="min-h-screen p-8">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="mb-8">
          <h1 className="text-4xl font-bold mb-2">
            🤖 ClawdBot Dashboard
          </h1>
          <p className="text-gray-400">
            Control your ORE mining and betting bots on Railway
          </p>
        </div>

        {/* Stats */}
        <Stats stats={stats} />

        {/* Bot Cards */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8">
          {bots.map(bot => (
            <BotCard
              key={bot.id}
              bot={bot}
              onStart={() => handleBotAction(bot.id, 'start')}
              onStop={() => handleBotAction(bot.id, 'stop')}
            />
          ))}
        </div>

        {/* Terminal */}
        <Terminal logs={logs} />
      </div>
    </main>
  )
}
