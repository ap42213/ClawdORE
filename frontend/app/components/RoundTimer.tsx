'use client'

import { useEffect, useState } from 'react'

interface RoundTimerProps {
  roundId: number
  timeRemaining?: number
  roundDuration?: number
  updatedAt?: string
}

export default function RoundTimer({ roundId, timeRemaining, roundDuration = 60, updatedAt }: RoundTimerProps) {
  const [displayTime, setDisplayTime] = useState(timeRemaining || 0)
  
  useEffect(() => {
    if (timeRemaining === undefined) return
    
    // Calculate how much time has passed since the update
    let adjustedTime = timeRemaining
    if (updatedAt) {
      const updateTime = new Date(updatedAt).getTime()
      const now = Date.now()
      const elapsedSecs = Math.floor((now - updateTime) / 1000)
      adjustedTime = Math.max(0, timeRemaining - elapsedSecs)
    }
    
    setDisplayTime(adjustedTime)
    
    // Count down locally
    const interval = setInterval(() => {
      setDisplayTime(prev => {
        if (prev <= 0) return roundDuration // Reset for new round
        return prev - 1
      })
    }, 1000)
    
    return () => clearInterval(interval)
  }, [timeRemaining, updatedAt, roundDuration])
  
  const minutes = Math.floor(displayTime / 60)
  const seconds = displayTime % 60
  
  // Mining decision thresholds - push as close to 0 as possible
  // Solana block time ~400ms, tx propagation ~500ms, signing ~100ms
  // Realistically need ~1-2s buffer for safety
  const TOO_LATE = 1.5        // High risk - may miss round
  const SIGN_DEADLINE = 3     // Sign and submit NOW
  const DECISION_TIME = 5     // Final decision window  
  const PREPARE_TIME = 10     // Start analyzing
  
  // Progress percentage (inverted - fills as time passes)
  const progress = roundDuration > 0 ? ((roundDuration - displayTime) / roundDuration) * 100 : 0
  
  // Color based on mining decision windows
  const getColor = () => {
    if (displayTime <= TOO_LATE) return 'text-gray-500'
    if (displayTime <= SIGN_DEADLINE) return 'text-red-400'
    if (displayTime <= DECISION_TIME) return 'text-orange-400'
    if (displayTime <= PREPARE_TIME) return 'text-yellow-400'
    return 'text-green-400'
  }
  
  const getBarColor = () => {
    if (displayTime <= TOO_LATE) return 'bg-gray-600'
    if (displayTime <= SIGN_DEADLINE) return 'bg-red-500'
    if (displayTime <= DECISION_TIME) return 'bg-orange-500'
    if (displayTime <= PREPARE_TIME) return 'bg-yellow-500'
    return 'bg-green-500'
  }
  
  const getMiningStatus = () => {
    if (displayTime <= TOO_LATE && displayTime > 0) return { icon: '💀', text: 'TOO LATE', urgent: true }
    if (displayTime <= SIGN_DEADLINE) return { icon: '🚨', text: 'SIGN NOW', urgent: true }
    if (displayTime <= DECISION_TIME) return { icon: '⚡', text: 'DECIDE', urgent: true }
    if (displayTime <= PREPARE_TIME) return { icon: '📊', text: 'ANALYZE', urgent: false }
    return null
  }
  
  const miningStatus = getMiningStatus()

  return (
    <div className="bg-slate-800/80 backdrop-blur rounded-xl p-4 border border-slate-700/50">
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center gap-2">
          <span className="text-2xl">⏱️</span>
          <div>
            <div className="text-xs text-gray-500 uppercase tracking-wide">Round #{roundId}</div>
            <div className={`text-2xl font-mono font-bold ${getColor()}`}>
              {minutes}:{seconds.toString().padStart(2, '0')}
            </div>
          </div>
        </div>
        <div className="text-right">
          <div className="text-xs text-gray-500">Round Duration</div>
          <div className="text-sm text-gray-400">~{roundDuration}s</div>
        </div>
      </div>
      
      {/* Progress bar */}
      <div className="h-2 bg-slate-700 rounded-full overflow-hidden">
        <div 
          className={`h-full ${getBarColor()} transition-all duration-1000 ease-linear`}
          style={{ width: `${progress}%` }}
        />
      </div>
      
      {miningStatus && (
        <div className={`mt-2 text-center text-xs ${miningStatus.urgent ? 'animate-pulse' : ''} ${getColor()}`}>
          {miningStatus.icon} {miningStatus.text}
        </div>
      )}
    </div>
  )
}
