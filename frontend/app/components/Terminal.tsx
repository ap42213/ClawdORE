import { Signal } from '../page'

interface TerminalProps {
  logs: string[]
  signals?: Signal[]
}

export default function Terminal({ logs, signals }: TerminalProps) {
  const formatTime = (timestamp: string) => {
    return new Date(timestamp).toLocaleTimeString()
  }

  const getSignalEmoji = (type: string) => {
    switch (type) {
      case 'Heartbeat': return '💓'
      case 'RoundStarted': return '🆕'
      case 'RoundEnded': return '🏁'
      case 'DeployDetected': return '📤'
      case 'WinDetected': return '🏆'
      default: return '📨'
    }
  }

  const getBotColor = (bot: string) => {
    const colors: Record<string, string> = {
      'coordinator-bot': 'text-purple-400',
      'monitor-bot': 'text-cyan-400',
      'analytics-bot': 'text-green-400',
      'parser-bot': 'text-yellow-400',
      'learning-bot': 'text-pink-400',
      'betting-bot': 'text-orange-400',
      'miner-bot': 'text-blue-400',
    }
    return colors[bot] || 'text-gray-400'
  }

  return (
    <div className="bg-slate-800/80 backdrop-blur rounded-xl p-4 border border-slate-700/50 h-full">
      <div className="flex items-center justify-between mb-3">
        <h3 className="text-lg font-bold text-gray-200">📡 Signal Feed</h3>
        <div className="flex gap-1.5">
          <div className="w-3 h-3 rounded-full bg-red-500" />
          <div className="w-3 h-3 rounded-full bg-yellow-500" />
          <div className="w-3 h-3 rounded-full bg-green-500" />
        </div>
      </div>
      
      <div className="terminal text-xs">
        {signals && signals.length > 0 ? (
          signals.slice(0, 20).map((signal, index) => (
            <div key={signal.id || index} className="terminal-line py-1 border-b border-slate-700/30">
              <span className="text-gray-600">{formatTime(signal.created_at)}</span>
              <span className="mx-2">{getSignalEmoji(signal.signal_type)}</span>
              <span className={`font-mono ${getBotColor(signal.source_bot)}`}>
                {signal.source_bot.replace('-bot', '').toUpperCase()}
              </span>
              <span className="text-gray-500 ml-2">{signal.signal_type}</span>
            </div>
          ))
        ) : (
          logs.map((log, index) => (
            <div key={index} className="terminal-line py-1">
              <span className="text-gray-600">[{new Date().toLocaleTimeString()}]</span>{' '}
              <span className="text-gray-300">{log}</span>
            </div>
          ))
        )}
      </div>
    </div>
  )
}
