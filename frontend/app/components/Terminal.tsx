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

  return (
    <div className="card signal-feed">
      <div className="card-header">
        <h3 className="card-title">Signal Feed</h3>
        <div className="terminal-dots">
          <div className="dot red" />
          <div className="dot yellow" />
          <div className="dot green" />
        </div>
      </div>
      
      <div className="terminal">
        {signals && signals.length > 0 ? (
          signals.slice(0, 20).map((signal, index) => (
            <div key={signal.id || index} className="terminal-line">
              <span className="terminal-time">{formatTime(signal.created_at)}</span>
              <span className="terminal-emoji">{getSignalEmoji(signal.signal_type)}</span>
              <span className="terminal-source">{signal.source_bot.replace('-bot', '').toUpperCase()}</span>
              <span className="terminal-type">{signal.signal_type}</span>
            </div>
          ))
        ) : (
          logs.map((log, index) => (
            <div key={index} className="terminal-line">
              <span className="terminal-time">[{new Date().toLocaleTimeString()}]</span>
              <span className="terminal-text">{log}</span>
            </div>
          ))
        )}
      </div>
    </div>
  )
}
