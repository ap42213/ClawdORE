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
  
  const totalDeployed = round?.deployed_squares?.reduce((a, b) => a + b, 0) || 0
  const maxDeployed = round?.deployed_squares ? Math.max(...round.deployed_squares) : 0

  const getCellClasses = (idx: number) => {
    const deployed = round?.deployed_squares?.[idx] || 0
    const isWinner = round?.winning_square === idx
    const hasDeploys = deployed > 0
    
    // Calculate heat level (1-5)
    const heatLevel = maxDeployed > 0 
      ? Math.ceil((deployed / maxDeployed) * 5) 
      : 0
    
    let classes = 'grid-cell'
    
    if (isWinner) {
      classes += ' winner'
    } else if (hasDeploys) {
      classes += ` has-deploys heat-${heatLevel}`
    }
    
    return classes
  }

  const getPercentage = (idx: number) => {
    if (!round?.deployed_squares?.[idx] || totalDeployed === 0) return null
    return ((round.deployed_squares[idx] / totalDeployed) * 100).toFixed(1)
  }

  const getDeployedSol = (idx: number) => {
    if (!round?.deployed_squares?.[idx]) return null
    const sol = round.deployed_squares[idx] / 1e9
    if (sol < 0.001) return null
    return sol.toFixed(3)
  }

  return (
    <div className="card">
      <div className="card-header">
        <h3 className="card-title">⛏️ Regolith Grid</h3>
        {round && (
          <span className="round-badge">Round #{round.round_id}</span>
        )}
      </div>
      
      <div className="grid-container">
        <div className="regolith-grid">
          {squares.map((idx) => {
            const deployedSol = getDeployedSol(idx)
            const percentage = getPercentage(idx)
            
            return (
              <div key={idx} className={getCellClasses(idx)}>
                <span className="cell-number">#{idx + 1}</span>
                {deployedSol ? (
                  <>
                    <span className="cell-amount">{deployedSol}</span>
                    {percentage && (
                      <span className="cell-percentage">{percentage}%</span>
                    )}
                  </>
                ) : (
                  <span className="cell-amount zero">—</span>
                )}
              </div>
            )
          })}
        </div>

        {/* Total deployed */}
        {round && (
          <div style={{ 
            textAlign: 'center',
            padding: '0.75rem',
            background: 'var(--bg-tertiary)',
            borderRadius: '10px',
            width: '100%',
            maxWidth: '500px'
          }}>
            <span style={{ color: 'var(--text-secondary)', fontSize: '0.875rem' }}>
              Total Deployed:{' '}
            </span>
            <span style={{ 
              color: 'var(--accent-primary)', 
              fontFamily: "'JetBrains Mono', monospace",
              fontWeight: 700,
              fontSize: '1.25rem'
            }}>
              {(totalDeployed / 1e9).toFixed(4)} SOL
            </span>
          </div>
        )}
      </div>
    </div>
  )
}
