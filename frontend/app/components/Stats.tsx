interface StatsProps {
  stats: {
    balance: string
    roundsWon: number
    totalDeployed: string
    activeBots: number
    currentRound: number
    playersTracked: number
    transactionsProcessed: number
  }
}

export default function Stats({ stats }: StatsProps) {
  const statCards = [
    { label: 'Current Round', value: `#${stats.currentRound.toLocaleString()}` },
    { label: 'Total Deployed', value: `${stats.totalDeployed} SOL` },
    { label: 'Players Tracked', value: stats.playersTracked.toLocaleString() },
    { label: 'Transactions', value: stats.transactionsProcessed.toLocaleString() },
    { label: 'Wins Tracked', value: stats.roundsWon.toLocaleString() },
    { label: 'Active Bots', value: `${stats.activeBots}/7` },
  ]

  return (
    <div className="stats-row">
      {statCards.map((stat, index) => (
        <div key={index} className="stat-card">
          <span className="stat-value">{stat.value}</span>
          <span className="stat-label">{stat.label}</span>
        </div>
      ))}
    </div>
  )
}
