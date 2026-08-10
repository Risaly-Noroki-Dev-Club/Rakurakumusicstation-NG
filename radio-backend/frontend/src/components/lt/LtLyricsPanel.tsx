import { useEffect, useRef } from 'react'
import { ScrollArea } from '@appica/ui-react/scroll-area'
import { useStore } from '@/store'
import { activeLyricsIndex, usePlaybackClock } from '@/hooks/usePlaybackClock'
import { cn } from '@/lib/cn'

/** 一起听 — shared lyrics panel with active-line highlight and auto-scroll. */
export function LtLyricsPanel() {
  const playback = useStore((s) => s.playback)
  const position = usePlaybackClock(playback)
  const listRef = useRef<HTMLDivElement>(null)
  const lastActiveRef = useRef(-1)
  const songKeyRef = useRef('')

  const lines = playback?.lyricsLines ?? []
  const active = activeLyricsIndex(lines, position)
  const songKey = playback ? `${playback.songId}|${playback.title}` : ''

  useEffect(() => {
    if (!listRef.current || active < 0) return
    if (active === lastActiveRef.current && songKey === songKeyRef.current) return
    lastActiveRef.current = active
    songKeyRef.current = songKey
    const el = listRef.current.children[active] as HTMLElement | undefined
    el?.scrollIntoView({ block: 'center', behavior: 'smooth' })
  }, [active, songKey])

  const hasLyrics = lines.length > 0

  return (
    <ScrollArea className="min-h-0 flex-1" scrollShadow>
      <div className="flex h-full flex-col justify-center px-4 py-6" ref={listRef}>
        {hasLyrics ? (
          lines.map((line, i) => (
            <p
              key={`${line.time_ms}-${i}`}
              className={cn(
                'py-1.5 text-center text-base transition-colors md:text-lg',
                i === active ? 'lyrics-active font-semibold' : 'lyrics-inactive',
              )}
            >
              {line.text || '\u00a0'}
            </p>
          ))
        ) : (
          <div className="flex flex-col items-center gap-2 text-center">
            <p className="text-foreground-intense text-lg font-medium">{playback?.title ?? '一起听歌'}</p>
            <p className="text-foreground-muted text-sm">
              {playback?.artist ? `${playback.artist} · 一起听歌，分享此刻` : '一起听歌，分享此刻'}
            </p>
          </div>
        )}
      </div>
    </ScrollArea>
  )
}
