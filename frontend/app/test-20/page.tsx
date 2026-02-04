'use client'

import { useState, useEffect, useRef } from 'react'

interface RoundData {
  round_id: number
  winning_square: number
  our_picks: number[]
  hit: boolean
  timestamp: string
}

const SQUARE_COUNT = 20

export default function Test20Page() {
  const [currentRound, setCurrentRound] = useState<number>(0)
  const [recommendedSquares, setRecommendedSquares] = useState<number[]>([])
  const [winningSquare, setWinningSquare] = useState<number | null>(null)
  const [roundHistory, setRoundHistory] = useState<RoundData[]>([])
  const [stats, setStats] = useState({ hits: 0, misses: 0, hitRate: 0, streak: 0, bestStreak: 0 })
  const [loading, setLoading] = useState(true)
  const [lastUpdate, setLastUpdate] = useState<string>('')
  const seenRoundsRef = useRef<Set<number>>(new Set())

  // Fetch live data
  useEffect(() => {
    const fetchData = async () => {
      try {
        // Fetch current state
        const stateRes = await fetch('/api/state')
        if (stateRes.ok) {
          const state = await stateRes.json()
          
          if (state.consensus_recommendation) {
            const rec = state.consensus_recommendation
            // Get squares (already 1-indexed from coordinator)
            const squares = rec.squares || []
            // Take first 20 squares
            setRecommendedSquares(squares.slice(0, SQUARE_COUNT))
          }
          
          if (state.current_round) {
            setCurrentRound(state.current_round.round_id || 0)
          }
        }

        // Fetch results
        const resultsRes = await fetch('/api/results')
        if (resultsRes.ok) {
          const data = await resultsRes.json()
          const results = data.results || []
          
          if (results.length > 0) {
            const latest = results[0]
            setWinningSquare(latest.winning_square)
            
            // Process new rounds
            for (const result of results.slice(0, 50)) {
              if (!seenRoundsRef.current.has(result.round_id)) {
                seenRoundsRef.current.add(result.round_id)
                
                // Simulate 20-square picks for historical data
                const simulatedPicks = recommendedSquares.length > 0 
                  ? recommendedSquares 
                  : Array.from({length: SQUARE_COUNT}, (_, i) => i + 1)
                
                const hit = simulatedPicks.includes(result.winning_square)
                
                setRoundHistory(prev => [{
                  round_id: result.round_id,
                  winning_square: result.winning_square,
                  our_picks: simulatedPicks,
                  hit,
                  timestamp: result.created_at || new Date().toISOString()
                }, ...prev].slice(0, 100))
              }
            }
          }
        }

        setLastUpdate(new Date().toLocaleTimeString())
        setLoading(false)
      } catch (e) {
        console.error('Fetch error:', e)
      }
    }

    fetchData()
    const interval = setInterval(fetchData, 3000)
    return () => clearInterval(interval)
  }, [recommendedSquares])

  // Calculate stats
  useEffect(() => {
    if (roundHistory.length === 0) return
    
    const hits = roundHistory.filter(r => r.hit).length
    const total = roundHistory.length
    const hitRate = total > 0 ? (hits / total * 100) : 0
    
    // Calculate current streak
    let streak = 0
    for (const r of roundHistory) {
      if (r.hit) streak++
      else break
    }
    
    // Calculate best streak
    let bestStreak = 0
    let currentStreak = 0
    for (const r of roundHistory) {
      if (r.hit) {
        currentStreak++
        bestStreak = Math.max(bestStreak, currentStreak)
      } else {
        currentStreak = 0
      }
    }
    
    setStats({ hits, misses: total - hits, hitRate, streak, bestStreak })
  }, [roundHistory])

  // Render 5x5 grid
  const renderGrid = () => {
    const squares = []
    for (let i = 1; i <= 25; i++) {
      const isRecommended = recommendedSquares.includes(i)
      const isWinner = winningSquare === i
      const isHit = isRecommended && isWinner
      
      let bgColor = 'bg-gray-800'
      let borderColor = 'border-gray-700'
      
      if (isHit) {
        bgColor = 'bg-green-500'
        borderColor = 'border-green-400'
      } else if (isWinner) {
        bgColor = 'bg-red-500'
        borderColor = 'border-red-400'
      } else if (isRecommended) {
        bgColor = 'bg-blue-600'
        borderColor = 'border-blue-400'
      }
      
      squares.push(
        <div
          key={i}
          className={`${bgColor} ${borderColor} border-2 rounded-lg flex items-center justify-center font-bold text-lg transition-all duration-300 aspect-square`}
        >
          {i}
        </div>
      )
    }
    return squares
  }

  return (
    <div className="min-h-screen bg-gray-900 text-white p-6">
      <div className="max-w-6xl mx-auto">
        {/* Header */}
        <div className="flex justify-between items-center mb-8">
          <div>
            <h1 className="text-3xl font-bold text-blue-400">🎯 20-Square Test Mode</h1>
            <p className="text-gray-400 mt-1">Testing {SQUARE_COUNT}/25 square coverage strategy</p>
          </div>
          <div className="text-right">
            <div className="text-2xl font-mono text-yellow-400">Round #{currentRound}</div>
            <div className="text-sm text-gray-500">Updated: {lastUpdate}</div>
          </div>
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-2 gap-8">
          {/* Left: Grid */}
          <div>
            <h2 className="text-xl font-bold mb-4">Live Board</h2>
            <div className="grid grid-cols-5 gap-2 mb-4">
              {renderGrid()}
            </div>
            <div className="flex gap-4 text-sm">
              <div className="flex items-center gap-2">
                <div className="w-4 h-4 bg-blue-600 rounded"></div>
                <span>Selected ({recommendedSquares.length})</span>
              </div>
              <div className="flex items-center gap-2">
                <div className="w-4 h-4 bg-red-500 rounded"></div>
                <span>Winner (missed)</span>
              </div>
              <div className="flex items-center gap-2">
                <div className="w-4 h-4 bg-green-500 rounded"></div>
                <span>HIT! 🎉</span>
              </div>
            </div>
          </div>

          {/* Right: Stats */}
          <div>
            <h2 className="text-xl font-bold mb-4">Win/Loss Statistics</h2>
            <div className="grid grid-cols-3 gap-4 mb-6">
              <div className="bg-gray-800 rounded-lg p-4 text-center">
                <div className="text-4xl font-bold text-green-400">{stats.hits}</div>
                <div className="text-gray-400 font-bold">WINS</div>
              </div>
              <div className="bg-gray-800 rounded-lg p-4 text-center">
                <div className="text-4xl font-bold text-red-400">{stats.misses}</div>
                <div className="text-gray-400 font-bold">LOSSES</div>
              </div>
              <div className="bg-gray-800 rounded-lg p-4 text-center">
                <div className="text-4xl font-bold text-yellow-400">{stats.hitRate.toFixed(1)}%</div>
                <div className="text-gray-400 font-bold">WIN %</div>
              </div>
            </div>
            
            <div className="grid grid-cols-2 gap-4 mb-6">
              <div className="bg-gray-800 rounded-lg p-4 text-center">
                <div className="text-2xl font-bold text-blue-400">{stats.hits + stats.misses}</div>
                <div className="text-gray-400">Total Rounds</div>
              </div>
              <div className="bg-gray-800 rounded-lg p-4 text-center">
                <div className="text-2xl font-bold text-purple-400">{stats.streak}</div>
                <div className="text-gray-400">Win Streak</div>
                <div className="text-xs text-gray-500">Best: {stats.bestStreak}</div>
              </div>
            </div>

            <div className="bg-gray-800 rounded-lg p-4 mb-4">
              <div className="text-lg font-bold mb-2">Coverage Analysis</div>
              <div className="text-sm text-gray-400">
                <p>Selecting {SQUARE_COUNT} of 25 squares = {(SQUARE_COUNT/25*100).toFixed(0)}% coverage</p>
                <p className="mt-1">Theoretical hit rate: {(SQUARE_COUNT/25*100).toFixed(0)}%</p>
                <p className="mt-1">Actual hit rate: {stats.hitRate.toFixed(1)}%</p>
                <p className={`mt-2 font-bold ${stats.hitRate >= 80 ? 'text-green-400' : 'text-yellow-400'}`}>
                  {stats.hitRate >= 80 ? '✅ On target!' : '⚠️ Below expected'}
                </p>
              </div>
            </div>

            <div className="bg-gray-800 rounded-lg p-4">
              <div className="text-lg font-bold mb-2">Selected Squares</div>
              <div className="flex flex-wrap gap-2">
                {recommendedSquares.map(sq => (
                  <span key={sq} className="bg-blue-600 px-2 py-1 rounded text-sm font-mono">
                    {sq}
                  </span>
                ))}
              </div>
            </div>
          </div>
        </div>

        {/* History */}
        <div className="mt-8">
          <h2 className="text-xl font-bold mb-4">Recent Rounds</h2>
          <div className="bg-gray-800 rounded-lg overflow-hidden">
            <div className="max-h-64 overflow-y-auto">
              <table className="w-full text-sm">
                <thead className="bg-gray-700 sticky top-0">
                  <tr>
                    <th className="px-4 py-2 text-left">Round</th>
                    <th className="px-4 py-2 text-left">Winner</th>
                    <th className="px-4 py-2 text-left">Result</th>
                    <th className="px-4 py-2 text-left">Time</th>
                  </tr>
                </thead>
                <tbody>
                  {roundHistory.slice(0, 20).map(r => (
                    <tr key={r.round_id} className={r.hit ? 'bg-green-900/30' : 'bg-red-900/30'}>
                      <td className="px-4 py-2 font-mono">#{r.round_id}</td>
                      <td className="px-4 py-2 font-mono">{r.winning_square}</td>
                      <td className="px-4 py-2">
                        {r.hit ? (
                          <span className="text-green-400">✅ HIT</span>
                        ) : (
                          <span className="text-red-400">❌ MISS</span>
                        )}
                      </td>
                      <td className="px-4 py-2 text-gray-400">
                        {new Date(r.timestamp).toLocaleTimeString()}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        </div>

        {/* Back link */}
        <div className="mt-8 text-center">
          <a href="/" className="text-blue-400 hover:underline">← Back to main dashboard</a>
        </div>
      </div>
    </div>
  )
}
