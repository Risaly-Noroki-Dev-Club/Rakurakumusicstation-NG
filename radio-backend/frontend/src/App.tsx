import { useEffect } from 'react'
import { Outlet, useLocation } from 'react-router-dom'
import { useTheme } from '@appica/ui-react/hooks/use-theme'
import { fetchMe, fetchStation } from '@/api'
import { connectWebSocket, startPollers } from '@/api/ws'
import { useStore } from '@/store'
import { MainNav } from '@/components/layout/MainNav'
import { ThemeToggle } from '@/components/layout/ThemeToggle'
import { MiniPlayer } from '@/components/layout/MiniPlayer'
import { Toasts } from '@/components/layout/Toasts'
import { StreamPlayer } from '@/components/StreamPlayer'

export default function App() {
  const station = useStore((s) => s.station)
  const accent = useStore((s) => s.accent)
  const location = useLocation()
  const { mounted, resolvedTheme } = useTheme()

  // Bootstrap: station info, identity, websocket, fallback pollers.
  useEffect(() => {
    let cancelled = false
    void fetchStation()
      .then((st) => {
        if (!cancelled) useStore.getState().setStation(st)
      })
      .catch(() => undefined)
    void fetchMe()
      .then((me) => {
        if (!cancelled) useStore.getState().setAuth(me)
      })
      .catch(() => undefined)
    connectWebSocket()
    const stop = startPollers()
    return () => {
      cancelled = true
      stop()
    }
  }, [])

  // Brand accent → CSS variable consumed by index.css token overrides.
  useEffect(() => {
    document.documentElement.style.setProperty('--accent', accent)
  }, [accent])

  // Title follows the station name.
  useEffect(() => {
    document.title = station?.name ? `${station.name} · Rakuraku` : 'Rakuraku Music Station'
  }, [station?.name])

  const isListen = location.pathname.startsWith('/listen')

  return (
    <div className={isListen ? 'lt-layout flex h-full flex-col' : 'flex h-full flex-col'}>
      <header className="border-border-muted bg-background/85 sticky top-0 z-40 border-b backdrop-blur">
        <div className="mx-auto flex h-14 w-full max-w-6xl items-center gap-2 px-4">
          <MainNav />
          <div className="ms-auto flex shrink-0 items-center gap-1">
            {mounted && <ThemeToggle resolvedTheme={resolvedTheme} />}
          </div>
        </div>
      </header>
      <main className="min-h-0 flex-1 overflow-y-auto">
        <Outlet />
      </main>
      <MiniPlayer />
      <StreamPlayer />
      <Toasts />
    </div>
  )
}
