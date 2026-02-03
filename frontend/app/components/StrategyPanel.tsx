interface Strategy {
  name: string
  squares: number[]
  weights: number[]
  reasoning: string
  confidence: number
  expected_roi: number
}

interface StrategyPanelProps {
  recommendation: {
    squares: number[]
    weights: string[]
    confidence: number
  } | null
  strategies?: Strategy[]
}

export default function StrategyPanel({ recommendation, strategies }: StrategyPanelProps) {
  if (!recommendation) {
    return (
      <div className="bg-slate-800/80 backdrop-blur rounded-xl p-4 border border-slate-700/50 h-full">
        <h3 className="text-lg font-bold text-gray-200 mb-3">🎯 Strategy</h3>
        <div className="flex items-center justify-center h-32">
          <p className="text-gray-500 text-sm">Waiting for recommendations...</p>
        </div>
      </div>
    )
  }

  const confidenceColor = recommendation.confidence >= 50 
    ? 'text-green-400' 
    : recommendation.confidence >= 30 
      ? 'text-yellow-400' 
      : 'text-red-400'

  const confidenceBarColor = recommendation.confidence >= 50 
    ? 'bg-green-500' 
    : recommendation.confidence >= 30 
      ? 'bg-yellow-500' 
      : 'bg-red-500'

  // Sort strategies by confidence
  const sortedStrategies = strategies 
    ? [...strategies].sort((a, b) => b.confidence - a.confidence)
    : []

  const medals = ['🥇', '🥈', '🥉', '📊']
  const medalColors = ['text-yellow-400', 'text-gray-400', 'text-orange-400', 'text-gray-500']

  return (
    <div className="bg-slate-800/80 backdrop-blur rounded-xl p-4 border border-slate-700/50 h-full">
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-lg font-bold text-gray-200">🎯 Consensus Strategy</h3>
        <span className={`text-sm font-bold ${confidenceColor}`}>
          {recommendation.confidence}%
        </span>
      </div>

      {/* Confidence Bar */}
      <div className="mb-4">
        <div className="flex justify-between text-xs text-gray-500 mb-1">
          <span>Confidence</span>
          <span>{recommendation.confidence}%</span>
        </div>
        <div className="h-2 bg-slate-700 rounded-full overflow-hidden">
          <div 
            className={`h-full ${confidenceBarColor} transition-all duration-500`}
            style={{ width: `${recommendation.confidence}%` }}
          />
        </div>
      </div>

      {/* Recommended Squares */}
      <div className="mb-4">
        <h4 className="text-sm text-gray-400 mb-2">Recommended Squares</h4>
        <div className="flex flex-wrap gap-2">
          {recommendation.squares.map((square, idx) => (
            <div 
              key={square}
              className={`px-3 py-2 rounded-lg text-center ${
                idx === 0 
                  ? 'bg-orange-500/30 border border-orange-500/50' 
                  : 'bg-slate-700/50 border border-slate-600/50'
              }`}
            >
              <div className="text-lg font-bold text-white">#{square}</div>
              <div className={`text-xs ${idx === 0 ? 'text-orange-300' : 'text-gray-400'}`}>
                {recommendation.weights[idx] || '?'}
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Strategy Info - Dynamic from database */}
      <div className="bg-slate-900/50 rounded-lg p-3">
        <h4 className="text-xs text-gray-500 uppercase tracking-wide mb-2">Active Strategies</h4>
        <div className="space-y-2 text-xs">
          {sortedStrategies.length > 0 ? (
            sortedStrategies.slice(0, 4).map((strat, idx) => (
              <div key={strat.name} className="flex items-start gap-2">
                <span className={medalColors[idx] || 'text-gray-500'}>{medals[idx] || '•'}</span>
                <div className="flex-1">
                  <div className="flex items-center justify-between">
                    <span className="text-gray-300">{strat.name}</span>
                    <span className={`font-bold ${strat.confidence >= 0.5 ? 'text-green-400' : strat.confidence >= 0.3 ? 'text-yellow-400' : 'text-gray-500'}`}>
                      {(strat.confidence * 100).toFixed(0)}%
                    </span>
                  </div>
                  <div className="text-gray-500 text-[10px] truncate" title={strat.reasoning}>
                    {strat.reasoning}
                  </div>
                </div>
              </div>
            ))
          ) : (
            <>
              <div className="flex items-center gap-2">
                <span className="text-yellow-400">🥇</span>
                <span className="text-gray-300">Contrarian Value</span>
                <span className="text-gray-500 ml-auto">60%</span>
              </div>
              <div className="flex items-center gap-2">
                <span className="text-gray-400">🥈</span>
                <span className="text-gray-300">Momentum</span>
                <span className="text-gray-500 ml-auto">50%</span>
              </div>
              <div className="flex items-center gap-2">
                <span className="text-orange-400">🥉</span>
                <span className="text-gray-300">Quadrant Analysis</span>
                <span className="text-gray-500 ml-auto">45%</span>
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  )
}
