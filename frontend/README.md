# ClawdBot Dashboard (Frontend)

Modern Next.js dashboard for controlling your ORE mining bots on Railway.

## Features

- 🎨 Beautiful, responsive UI with Tailwind CSS
- 🤖 Control 4 specialized bots (Monitor, Analytics, Miner, Betting)
- 📊 Real-time statistics dashboard
- 📟 Live terminal logs
- ⚡ Fast and optimized for Vercel

## Quick Start

### Local Development

```bash
cd frontend
npm install
npm run dev
```

Open [http://localhost:3000](http://localhost:3000)

### Environment Variables

Create `.env.local`:

```env
NEXT_PUBLIC_API_URL=https://your-railway-backend.railway.app
```

## Deploy to Vercel

### Option 1: Vercel CLI

```bash
npm i -g vercel
vercel login
vercel
```

### Option 2: GitHub Integration

1. Push to GitHub
2. Go to [vercel.com](https://vercel.com)
3. Click "Import Project"
4. Select your repository
5. Set environment variable:
   - `NEXT_PUBLIC_API_URL`: Your Railway backend URL
6. Deploy!

### Option 3: One-Click Deploy

[![Deploy with Vercel](https://vercel.com/button)](https://vercel.com/new)

## Project Structure

```
frontend/
├── app/
│   ├── components/
│   │   ├── BotCard.tsx      # Individual bot control card
│   │   ├── Stats.tsx         # Statistics dashboard
│   │   └── Terminal.tsx      # Log terminal display
│   ├── globals.css           # Global styles
│   ├── layout.tsx            # Root layout
│   └── page.tsx              # Main dashboard page
├── public/                   # Static assets
├── package.json
├── tsconfig.json
└── next.config.js
```

## Components

### BotCard
Displays bot status and control buttons (Start/Stop).

### Stats
Shows real-time statistics:
- Wallet Balance
- Rounds Won
- Total Mined
- Active Bots

### Terminal
Live log viewer with timestamps and emoji indicators.

## API Integration

The frontend connects to your Railway backend:

```typescript
const API_URL = process.env.NEXT_PUBLIC_API_URL
await fetch(`${API_URL}/api/bots/${botId}/start`, { method: 'POST' })
```

## Customization

### Colors

Edit `tailwind.config.ts`:

```typescript
theme: {
  extend: {
    colors: {
      primary: '#6366f1',    // Change to your brand color
      secondary: '#8b5cf6',
    },
  },
}
```

### Add More Bots

Edit `app/page.tsx`:

```typescript
const [bots, setBots] = useState<Bot[]>([
  // Add your custom bot here
  {
    id: 'custom',
    name: 'Custom Bot',
    status: 'stopped',
    description: 'My custom bot',
    icon: '🔥',
  },
])
```

## Production Ready

- ✅ TypeScript for type safety
- ✅ Tailwind CSS for styling
- ✅ Responsive design
- ✅ Error handling
- ✅ Loading states
- ✅ Optimized for Vercel Edge Network

## Monitoring

- View logs in Vercel dashboard
- Monitor function execution times
- Track API errors
- Analytics integration ready

## Next Steps

1. Connect to real Railway backend API
2. Add WebSocket for live updates
3. Implement authentication
4. Add more detailed statistics
5. Create bot configuration UI

## Support

Questions? Check out:
- [Next.js Docs](https://nextjs.org/docs)
- [Vercel Docs](https://vercel.com/docs)
- [Tailwind CSS](https://tailwindcss.com/docs)
