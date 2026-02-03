import { RoundData } from '../page'

interface BoardGridProps {
  round: RoundData | null
  recommendation: {
    squares: number[]
    weights: string[]
    confidence: number
  } | null
}

export default function BoardGrid({ round, recommendation }: BoardGridProps) {
  const squares = Array.from({ length: 25 }, (_, i) => i)
  
  const getSquareColor = (idx: number) => {
    if (!round) return 'bg-slate-700/50'
    
    const deployed = round.deployed_squares?.[idx] || 0
    const deployedSol = deployed / 1e9
    
    // Winning square
    if (round.winning_square === idx) {
      return 'bg-yellow-500 animate-pulse'
    }
    
    // Recommended squares
    if (recommendation?.squares?.includes(idx)) {
      const weight = recommendation.squares.indexOf(idx)
      if (weight === 0) return 'bg-orange-500/80 ring-2 ring-orange-400'
      return 'bg-orange-500/40 ring-1 ring-orange-400/50'
    }
    
    // Based on deployment
    if (deployedSol > 1) return 'bg-green-600/60'
    if (deployedSol > 0.5) return 'bg-green-500/40'
    if (deployedSol > 0.1) return 'bg-green-400/20'
    if (deployedSol > 0) return 'bg-green-300/10'
    
    return 'bg-slate-700/30'
  }

  const getDeployed = (idx: number) => {
    if (!round?.deployed_squares?.[idx]) return null
    const sol = round.deployed_squares[idx] / 1e9
    if (sol < 0.01) return null
    return sol.toFixed(2)
  }

  return (
    <div className="bg-slate-800/80 backdrop-blur rounded-xl p-4 border border-slate-700/50">
      <div className="flex items-center justify-between mb-3">
        <h3 className="text-lg font-bold text-gray-200">🎮 ORE Board</h3>
        {round && (
          <span className="text-xs text-orange-400 font-mono">
            Round #{round.round_id}
          </span>
        )}
      </div>
      
      <div className="grid grid-cols-5 gap-1.5">
        {squares.map((idx) => (
          <div
            key={idx}
            className={`aspect-square rounded-lg flex flex-col items-center justify-center text-xs font-mono transition-all ${getSquareColor(idx)} hover:scale-105`}
          >
            <span className="text-gray-400 text-[10px]">#{idx}</span>
            {getDeployed(idx) && (
              <span className="text-green-300 text-[10px] font-bold">
                {getDeployed(idx)}
              </span>
            )}
          </div>
        ))}
      </div>

      {/* Legend */}
      <div className="mt-3 pt-3 border-t border-slate-700/50 flex flex-wrap gap-2 text-[10px]">
        <span className="flex items-center gap-1">
          <div className="w-3 h-3 rounded bg-orange-500/80 ring-1 ring-orange-400"></div>
          <span className="text-gray-400">Recommended</span>
        </span>
        <span className="flex items-center gap-1">
          <div className="w-3 h-3 rounded bg-green-500/40"></div>
          <span className="text-gray-400">Active</span>
        </span>
        <span className="flex items-center gap-1">
          <div className="w-3 h-3 rounded bg-yellow-500 animate-pulse"></div>
          <span className="text-gray-400">Winner</span>
        </span>
      </div>

      {/* Total */}
      {round && (
        <div className="mt-2 text-center">
          <span className="text-gray-400 text-xs">Total: </span>
          <span className="text-green-400 font-bold">
            {(round.total_deployed / 1e9).toFixed(4)} SOL
          </span>
        </div>
      )}
    </div>
  )
}
