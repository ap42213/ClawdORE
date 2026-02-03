import { Bot } from '../page'

interface BotCardProps {
  bot: Bot
}

export default function BotCard({ bot }: BotCardProps) {
  const statusColors = {
    online: 'bg-green-500',
    offline: 'bg-red-500',
    syncing: 'bg-yellow-500',
  }

  const statusGlow = {
    online: 'shadow-green-500/50',
    offline: 'shadow-red-500/30',
    syncing: 'shadow-yellow-500/50',
  }

  return (
    <div className={`bot-card bg-slate-800/80 backdrop-blur rounded-xl p-4 border border-slate-700/50 hover:border-orange-500/50 transition-all duration-300`}>
      {/* Status indicator */}
      <div className="flex items-center justify-between mb-3">
        <span className="text-3xl">{bot.icon}</span>
        <div className={`w-3 h-3 rounded-full ${statusColors[bot.status]} ${bot.status === 'online' ? 'animate-pulse' : ''} shadow-lg ${statusGlow[bot.status]}`} />
      </div>

      {/* Name */}
      <h3 className="text-sm font-bold text-orange-400 tracking-wide mb-1">
        {bot.displayName}
      </h3>
      
      {/* Description */}
      <p className="text-gray-500 text-xs leading-tight">
        {bot.description}
      </p>

      {/* Status text */}
      <div className="mt-3 pt-2 border-t border-slate-700/50">
        <span className={`text-xs uppercase tracking-wider ${
          bot.status === 'online' ? 'text-green-400' : 
          bot.status === 'syncing' ? 'text-yellow-400' : 'text-gray-500'
        }`}>
          {bot.status}
        </span>
      </div>
    </div>
  )
}
