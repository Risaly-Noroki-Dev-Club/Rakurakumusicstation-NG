import { useState } from 'react'
import { Button } from '@appica/ui-react/button'
import { Spinner } from '@appica/ui-react/spinner'
import { FileText, Heart, HeartFilled, PlayerPlay } from '@appica/icons-react'
import { addToQueue, coverUrl } from '@/api'
import { SongArtwork } from '@/components/SongArtwork'
import { formatTime } from '@/lib/format'
import { useStore } from '@/store'
import type { SongSummary } from '@/types'

export interface SongRowProps {
  song: SongSummary
}

function errorMessage(e: unknown): string {
  return e instanceof Error ? e.message : '操作失败'
}

/**
 * One song row: cover, title/artist, duration, lyrics indicator and row
 * actions (点歌 / 本地收藏). Favorites live in localStorage via the store.
 */
export function SongRow({ song }: SongRowProps) {
  const addToast = useStore((s) => s.addToast)
  const favoriteSongs = useStore((s) => s.favoriteSongs)
  const toggleFavorite = useStore((s) => s.toggleFavorite)
  const favorited = favoriteSongs.some((f) => f.song.id === song.id)
  const [queuing, setQueuing] = useState(false)

  const handleQueue = async () => {
    if (queuing) return
    setQueuing(true)
    try {
      await addToQueue(song.id)
      addToast('已加入点歌队列', 'success')
    } catch (e) {
      addToast(errorMessage(e), 'error')
    } finally {
      setQueuing(false)
    }
  }

  return (
    <div className="flex w-full items-center gap-2.5 rounded-lg px-2 py-2 transition-colors hover:bg-background-subtle sm:gap-3">
      <SongArtwork hasCover={song.has_cover} coverSrc={coverUrl(song.id)} size="md" className="shrink-0" />
      <div className="min-w-0 flex-1">
        <div className="flex min-w-0 items-center gap-1.5">
          <p className="min-w-0 truncate text-sm font-medium">{song.title}</p>
          {song.has_lyrics && (
            <FileText aria-label="有歌词" className="size-3.5 shrink-0 text-foreground-subtle" />
          )}
        </div>
        <p className="min-w-0 truncate text-xs text-foreground-muted">
          {song.artist}
          {song.album ? ` · ${song.album}` : ''}
        </p>
      </div>
      <span className="shrink-0 text-xs tabular-nums text-foreground-subtle">{formatTime(song.duration_ms)}</span>
      <div className="flex shrink-0 items-center">
        <Button variant="ghost" size="icon-md" aria-label="点歌" disabled={queuing} onClick={() => void handleQueue()}>
          {queuing ? <Spinner className="size-4.5" currentColor /> : <PlayerPlay />}
        </Button>
        <Button
          variant="ghost"
          size="icon-md"
          aria-label={favorited ? '取消收藏' : '收藏'}
          aria-pressed={favorited}
          onClick={() => toggleFavorite(song)}
        >
          {favorited ? <HeartFilled className="text-error" /> : <Heart />}
        </Button>
      </div>
    </div>
  )
}
