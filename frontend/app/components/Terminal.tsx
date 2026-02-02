interface TerminalProps {
  logs: string[]
}

export default function Terminal({ logs }: TerminalProps) {
  return (
    <div className="bg-slate-800 rounded-lg p-6 border border-slate-700">
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-xl font-semibold">📟 Terminal</h3>
        <div className="flex gap-2">
          <div className="w-3 h-3 rounded-full bg-red-500" />
          <div className="w-3 h-3 rounded-full bg-yellow-500" />
          <div className="w-3 h-3 rounded-full bg-green-500" />
        </div>
      </div>
      <div className="terminal">
        {logs.map((log, index) => (
          <div key={index} className="terminal-line">
            <span className="text-gray-500">[{new Date().toLocaleTimeString()}]</span>{' '}
            <span>{log}</span>
          </div>
        ))}
      </div>
    </div>
  )
}
