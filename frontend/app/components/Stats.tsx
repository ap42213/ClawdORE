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
    {
      label: 'Current Round',
      value: `#${stats.currentRound.toLocaleString()}`,
      icon: '🎯',
      color: 'from-orange-500 to-yellow-500',
    },
    {
      label: 'Total Deployed',
      value: `${stats.totalDeployed} SOL`,
      icon: '💰',
      color: 'from-green-500 to-emerald-500',
    },
    {
      label: 'Players Tracked',
      value: stats.playersTracked.toLocaleString(),
      icon: '👥',
      color: 'from-blue-500 to-cyan-500',
    },
    {
      label: 'Transactions',
      value: stats.transactionsProcessed.toLocaleString(),
      icon: '📊',
      color: 'from-purple-500 to-pink-500',
    },
    {
      label: 'Wins Tracked',
      value: stats.roundsWon.toLocaleString(),
      icon: '🏆',
      color: 'from-yellow-500 to-orange-500',
    },
    {
      label: 'Active Bots',
      value: `${stats.activeBots}/7`,
      icon: '🤖',
      color: 'from-indigo-500 to-purple-500',
    },
  ]

  return (
    <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-3 mb-6">
      {statCards.map((stat, index) => (
        <div
          key={index}
          className="bg-slate-800/60 backdrop-blur rounded-xl p-4 border border-slate-700/50 hover:border-slate-600/50 transition-all"
        >
          <div className="flex items-center gap-2 mb-2">
            <span className="text-xl">{stat.icon}</span>
            <span className={`text-lg font-bold bg-gradient-to-r ${stat.color} bg-clip-text text-transparent`}>
              {stat.value}
            </span>
          </div>
          <p className="text-gray-500 text-xs uppercase tracking-wide">{stat.label}</p>
        </div>
      ))}
    </div>
  )
}
