import { useEffect, useRef } from 'react'
import { FileMusic } from '@appica/icons-react'
import type { Playback } from '@/store'
import { activeLyricsIndex } from '@/hooks/usePlaybackClock'
import { cn } from '@/lib/cn'

export interface LyricsPanelProps {
  /** Current playback snapshot (store.playback). */
  playback: Playback | null
  /** Smoothed playback position in ms (usePlaybackClock output). */
  positionMs: number
  /** Applied to the scroll container — use to cap/override the height. */
  className?: string
}

/**
 * Karaoke-style lyrics panel. The active line is highlighted via the
 * `.lyrics-active` / `.lyrics-inactive` classes (index.css) and auto-scrolled
 * into view (smooth, only when the active index changes).
 *
 * States: `lyricsLines === null` → subtle placeholder (lyrics not delivered
 * yet); `[]` → "no lyrics" empty state; otherwise the line list.
 */
export function LyricsPanel({ playback, positionMs, className }: LyricsPanelProps) {
  const lines = playback?.lyricsLines ?? null
  const activeIndex = activeLyricsIndex(lines, positionMs)
  const containerRef = useRef<HTMLDivElement>(null)
  const songKey = playback ? `${playback.songId}|${playback.title}|${playback.streamUrl}` : ''
  const lastActiveRef = useRef<{ key: string; index: number }>({ key: '', index: -2 })

  useEffect(() => {
    if (activeIndex < 0) return
    const prev = lastActiveRef.current
    if (prev.key === songKey && prev.index === activeIndex) return
    lastActiveRef.current = { key: songKey, index: activeIndex }
    const line = containerRef.current?.querySelector<HTMLElement>('[data-lyrics-active="true"]')
    // scrollIntoView walks up to whichever ancestor actually scrolls
    // (the panel's own overflow container, or a wrapping ScrollArea).
    line?.scrollIntoView({ block: 'center', behavior: 'smooth' })
  }, [activeIndex, songKey])

  if (!lines) {
    // lines === null: the full lyrics frame hasn't arrived (or the server
    // never re-sends it after a WS reconnect for the same song). If the song
    // change was already observed (lyricsKnown), treat it as no-lyrics
    // instead of showing an infinite loading state.
    const noLyrics = playback?.lyricsKnown === true
    return (
      <div
        className={cn(
          'flex min-h-28 items-center justify-center rounded-xl border border-dashed border-border-muted px-4 py-8',
          className,
        )}
      >
        <p className="text-foreground-subtle text-sm">
          {noLyrics ? '当前歌曲暂无歌词' : playback ? '歌词加载中…' : '等待开播…'}
        </p>
      </div>
    )
  }

  if (lines.length === 0) {
    return (
      <div
        className={cn(
          'flex min-h-28 flex-col items-center justify-center gap-2 rounded-xl border border-dashed border-border-muted px-4 py-8',
          className,
        )}
      >
        <FileMusic className="text-foreground-subtle size-5" />
        <p className="text-foreground-subtle text-sm">当前歌曲暂无歌词</p>
      </div>
    )
  }

  return (
    <div
      ref={containerRef}
      role="region"
      aria-label="歌词"
      className={cn('max-h-[45vh] overflow-y-auto px-1 py-1', className)}
    >
      {lines.map((line, i) => (
        <p
          key={i}
          data-lyrics-active={i === activeIndex}
          aria-current={i === activeIndex || undefined}
          className={cn(
            'py-1.5 text-sm leading-relaxed transition-colors',
            i === activeIndex
              ? 'lyrics-active text-foreground-intense font-medium'
              : 'lyrics-inactive',
          )}
        >
          {line.text || '\u00a0'}
        </p>
      ))}
    </div>
  )
}
