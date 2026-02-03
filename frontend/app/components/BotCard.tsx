import { Bot } from '../page'

interface BotCardProps {
  bot: Bot
}

export default function BotCard({ bot }: BotCardProps) {
  return (
    <div className={`bot-card ${bot.status}`}>
      <div className="bot-header">
        <span className="bot-icon">{bot.icon}</span>
        <div className={`bot-status-dot ${bot.status}`} />
      </div>

      <h3 className="bot-name">{bot.displayName}</h3>
      <p className="bot-description">{bot.description}</p>

      <div className="bot-footer">
        <span className={`bot-status-text ${bot.status}`}>{bot.status}</span>
      </div>
    </div>
  )
}
