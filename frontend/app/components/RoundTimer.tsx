'use client'

import { useEffect, useState } from 'react'

interface RoundTimerProps {
  roundId: number
  timeRemaining?: number
  roundDuration?: number
  updatedAt?: string
  slotsRemaining?: number
}

export default function RoundTimer({ roundId, timeRemaining, roundDuration = 60, updatedAt, slotsRemaining }: RoundTimerProps) {
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
  const isUrgent = displayTime < 10
  
  // Progress percentage
  const progress = roundDuration > 0 ? ((roundDuration - displayTime) / roundDuration) * 100 : 0

  return (
    <div className="timer-container">
      <span className="timer-label">Round Ends In</span>
      <span className={`timer-value ${isUrgent ? 'urgent' : ''}`}>
        {minutes.toString().padStart(2, '0')}:{seconds.toString().padStart(2, '0')}
      </span>
      <div className="timer-progress">
        <div 
          className="timer-progress-bar"
          style={{ width: `${progress}%` }}
        />
      </div>
      {slotsRemaining !== undefined && (
        <span className="timer-slots">{slotsRemaining} slots remaining</span>
      )}
    </div>
  )
}
