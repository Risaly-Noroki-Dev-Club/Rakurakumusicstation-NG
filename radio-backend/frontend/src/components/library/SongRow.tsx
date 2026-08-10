import { useState } from 'react'
import { Button } from '@appica/ui-react/button'
import { Badge } from '@appica/ui-react/badge'
import { Spinner } from '@appica/ui-react/spinner'
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
} from '@appica/ui-react/dropdown-menu'
import { FileText, Heart, HeartFilled, PlayerPlay, PlaylistAdd } from '@appica/icons-react'
import { addSongToPlaylist, addToQueue, coverUrl, fetchPlaylists } from '@/api'
import { SongArtwork } from '@/components/SongArtwork'
import { formatTime } from '@/lib/format'
import { useStore, type Toast } from '@/store'
import type { PlaylistWithCount, SongSummary } from '@/types'

export interface SongRowProps {
  song: SongSummary
  favorited: boolean
  onToggleFavorite: (songId: number) => void
}

function errorMessage(e: unknown): string {
  return e instanceof Error ? e.message : '操作失败'
}

/**
 * One library song row: cover, title/artist, duration, lyrics indicator and
 * row actions (点歌 / 收藏 / 添加到播放列表).
 */
export function SongRow({ song, favorited, onToggleFavorite }: SongRowProps) {
  // store.ts is FINAL; its Toast level union omits 'success' but Toasts.tsx
  // renders a success variant — widen the binding here so call sites stay clean.
  const addToast = useStore((s) => s.addToast) as (
    message: string,
    level?: Toast['level'] | 'success',
  ) => void
  const [queuing, setQueuing] = useState(false)
  const [menuOpen, setMenuOpen] = useState(false)
  const [playlists, setPlaylists] = useState<PlaylistWithCount[] | null>(null)
  const [addingTo, setAddingTo] = useState<number | null>(null)

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

  const handleMenuOpenChange = (open: boolean) => {
    setMenuOpen(open)
    if (open && playlists === null) {
      fetchPlaylists()
        .then(setPlaylists)
        .catch((e) => {
          setPlaylists([])
          addToast(errorMessage(e), 'error')
        })
    }
  }

  const handleAddToPlaylist = async (playlistId: number) => {
    if (addingTo !== null) return
    setAddingTo(playlistId)
    try {
      await addSongToPlaylist(playlistId, song.id)
      addToast('已添加到播放列表', 'success')
    } catch (e) {
      addToast(errorMessage(e), 'error')
    } finally {
      setAddingTo(null)
      setMenuOpen(false)
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
          onClick={() => onToggleFavorite(song.id)}
        >
          {favorited ? <HeartFilled className="text-error" /> : <Heart />}
        </Button>
        <DropdownMenu open={menuOpen} onOpenChange={handleMenuOpenChange}>
          <DropdownMenuTrigger render={<Button variant="ghost" size="icon-md" aria-label="添加到播放列表" />}>
            <PlaylistAdd />
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="max-h-72 w-56">
            {playlists === null ? (
              <DropdownMenuItem disabled>
                <Spinner className="size-4" currentColor />
                加载中…
              </DropdownMenuItem>
            ) : playlists.length === 0 ? (
              <DropdownMenuItem disabled>暂无播放列表</DropdownMenuItem>
            ) : (
              playlists.map((p) => (
                <DropdownMenuItem
                  key={p.id}
                  disabled={addingTo === p.id}
                  onClick={() => void handleAddToPlaylist(p.id)}
                  className="justify-between gap-3"
                >
                  <span className="min-w-0 truncate">{p.name}</span>
                  <Badge variant="soft" size="xs" className="shrink-0">
                    {p.song_count}
                  </Badge>
                </DropdownMenuItem>
              ))
            )}
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </div>
  )
}
