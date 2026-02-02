interface StatsProps {
  stats: {
    balance: string
    roundsWon: number
    totalMined: string
    activeBots: number
  }
}

export default function Stats({ stats }: StatsProps) {
  const statCards = [
    {
      label: 'Wallet Balance',
      value: `${stats.balance} SOL`,
      icon: '💰',
      color: 'text-green-400',
    },
    {
      label: 'Rounds Won',
      value: stats.roundsWon,
      icon: '🏆',
      color: 'text-yellow-400',
    },
    {
      label: 'Total Mined',
      value: `${stats.totalMined} ORE`,
      icon: '⛏️',
      color: 'text-blue-400',
    },
    {
      label: 'Active Bots',
      value: `${stats.activeBots}/4`,
      icon: '🤖',
      color: 'text-purple-400',
    },
  ]

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8">
      {statCards.map((stat, index) => (
        <div
          key={index}
          className="bg-slate-800 rounded-lg p-6 border border-slate-700"
        >
          <div className="flex items-center justify-between mb-2">
            <span className="text-2xl">{stat.icon}</span>
            <span className={`text-2xl font-bold ${stat.color}`}>
              {stat.value}
            </span>
          </div>
          <p className="text-gray-400 text-sm">{stat.label}</p>
        </div>
      ))}
    </div>
  )
}
