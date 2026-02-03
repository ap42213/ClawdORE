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
      <div className="card strategy-panel">
        <h3 className="card-title">🎯 Strategy</h3>
        <div className="strategy-waiting">
          <p>Waiting for recommendations...</p>
        </div>
      </div>
    )
  }

  const confidenceLevel = recommendation.confidence >= 50 
    ? 'high' 
    : recommendation.confidence >= 30 
      ? 'medium' 
      : 'low'

  // Sort strategies by confidence
  const sortedStrategies = strategies 
    ? [...strategies].sort((a, b) => b.confidence - a.confidence)
    : []

  const medals = ['🥇', '🥈', '🥉', '📊']

  return (
    <div className="card strategy-panel">
      <div className="card-header">
        <h3 className="card-title">🎯 Consensus Strategy</h3>
        <span className={`confidence-badge ${confidenceLevel}`}>
          {recommendation.confidence}%
        </span>
      </div>

      {/* Confidence Bar */}
      <div className="confidence-section">
        <div className="confidence-labels">
          <span>Confidence</span>
          <span>{recommendation.confidence}%</span>
        </div>
        <div className="confidence-bar">
          <div 
            className={`confidence-fill ${confidenceLevel}`}
            style={{ width: `${recommendation.confidence}%` }}
          />
        </div>
      </div>

      {/* Recommended Squares */}
      <div className="recommended-squares">
        <h4 className="section-label">Recommended Squares</h4>
        <div className="squares-list">
          {recommendation.squares.map((square, idx) => (
            <div 
              key={square}
              className={`square-badge ${idx === 0 ? 'primary' : ''}`}
            >
              <div className="square-number">#{square}</div>
              <div className="square-weight">
                {recommendation.weights[idx] || '?'}
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Strategy Info - Dynamic from database */}
      <div className="strategies-section">
        <h4 className="section-label">Active Strategies</h4>
        <div className="strategies-list">
          {sortedStrategies.length > 0 ? (
            sortedStrategies.slice(0, 4).map((strat, idx) => (
              <div key={strat.name} className="strategy-item">
                <span className="strategy-medal">{medals[idx] || '•'}</span>
                <div className="strategy-content">
                  <div className="strategy-header">
                    <span className="strategy-name">{strat.name}</span>
                    <span className={`strategy-confidence ${strat.confidence >= 0.5 ? 'high' : strat.confidence >= 0.3 ? 'medium' : 'low'}`}>
                      {(strat.confidence * 100).toFixed(0)}%
                    </span>
                  </div>
                  <div className="strategy-reasoning" title={strat.reasoning}>
                    {strat.reasoning}
                  </div>
                </div>
              </div>
            ))
          ) : (
            <>
              <div className="strategy-item">
                <span className="strategy-medal">🥇</span>
                <span className="strategy-name">Contrarian Value</span>
                <span className="strategy-confidence">60%</span>
              </div>
              <div className="strategy-item">
                <span className="strategy-medal">🥈</span>
                <span className="strategy-name">Momentum</span>
                <span className="strategy-confidence">50%</span>
              </div>
              <div className="strategy-item">
                <span className="strategy-medal">🥉</span>
                <span className="strategy-name">Quadrant Analysis</span>
                <span className="strategy-confidence">45%</span>
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  )
}
