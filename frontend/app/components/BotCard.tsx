import { Bot } from '../page'

interface BotCardProps {
  bot: Bot
  onStart: () => void
  onStop: () => void
}

export default function BotCard({ bot, onStart, onStop }: BotCardProps) {
  const statusColors = {
    running: 'bg-green-500',
    stopped: 'bg-red-500',
    starting: 'bg-yellow-500',
  }

  return (
    <div className="bot-card bg-slate-800 rounded-lg p-6 border border-slate-700">
      {/* Header */}
      <div className="flex items-center justify-between mb-4">
        <span className="text-4xl">{bot.icon}</span>
        <div className="flex items-center">
          <span className={`status-indicator status-${bot.status}`} />
          <span className="text-sm text-gray-400 capitalize">{bot.status}</span>
        </div>
      </div>

      {/* Content */}
      <h3 className="text-xl font-semibold mb-2">{bot.name}</h3>
      <p className="text-gray-400 text-sm mb-4">{bot.description}</p>

      {/* Actions */}
      <div className="flex gap-2">
        <button
          onClick={onStart}
          disabled={bot.status !== 'stopped'}
          className={`flex-1 py-2 px-4 rounded font-medium transition ${
            bot.status === 'stopped'
              ? 'bg-green-600 hover:bg-green-700 text-white'
              : 'bg-gray-700 text-gray-500 cursor-not-allowed'
          }`}
        >
          Start
        </button>
        <button
          onClick={onStop}
          disabled={bot.status === 'stopped'}
          className={`flex-1 py-2 px-4 rounded font-medium transition ${
            bot.status !== 'stopped'
              ? 'bg-red-600 hover:bg-red-700 text-white'
              : 'bg-gray-700 text-gray-500 cursor-not-allowed'
          }`}
        >
          Stop
        </button>
      </div>
    </div>
  )
}
