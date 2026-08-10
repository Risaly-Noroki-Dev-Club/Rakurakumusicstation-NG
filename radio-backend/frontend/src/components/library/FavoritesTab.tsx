import { useEffect, useState } from 'react'
import { Button } from '@appica/ui-react/button'
import { Skeleton } from '@appica/ui-react/skeleton'
import { Bookmark, HeartFilled, X } from '@appica/icons-react'
import { fetchFavorites, fetchPlaylists, fetchSong, removeFavorite } from '@/api'
import { SongRow } from '@/components/library/SongRow'
import { useStore, type Toast } from '@/store'
import type { Favorite, PlaylistWithCount, SongSummary } from '@/types'

function errorMessage(e: unknown): string {
  return e instanceof Error ? e.message : '操作失败'
}

/** 收藏 tab：收藏的歌曲（复用 SongRow）+ 收藏的播放列表。 */
export function FavoritesTab() {
  // store.ts is FINAL; its Toast level union omits 'success' but Toasts.tsx
  // renders a success variant — widen the binding here so call sites stay clean.
  const addToast = useStore((s) => s.addToast) as (
    message: string,
    level?: Toast['level'] | 'success',
  ) => void
  const [favorites, setFavorites] = useState<Favorite[] | null>(null)
  const [songData, setSongData] = useState<Map<number, SongSummary>>(new Map())
  const [playlists, setPlaylists] = useState<PlaylistWithCount[]>([])

  useEffect(() => {
    let cancelled = false
    fetchFavorites()
      .then((favs) => {
        if (cancelled) return
        setFavorites(favs)
        const songIds = favs.filter((f) => f.song_id != null).map((f) => f.song_id as number)
        return Promise.allSettled(songIds.map((id) => fetchSong(id))).then((results) => {
          if (cancelled) return
          const map = new Map<number, SongSummary>()
          results.forEach((r, i) => {
            if (r.status === 'fulfilled') map.set(songIds[i], r.value)
          })
          setSongData(map)
        })
      })
      .catch(() => {
        if (cancelled) return
        setFavorites([])
        addToast('获取收藏失败', 'error')
      })
    return () => {
      cancelled = true
    }
  }, [addToast])

  // 用于解析收藏的播放列表名称。
  useEffect(() => {
    let cancelled = false
    fetchPlaylists()
      .then((list) => {
        if (!cancelled) setPlaylists(list)
      })
      .catch(() => undefined)
    return () => {
      cancelled = true
    }
  }, [])

  const unfavorite = async (fav: Favorite) => {
    const prev = favorites
    setFavorites((cur) => (cur ?? []).filter((f) => f.id !== fav.id))
    try {
      await removeFavorite(fav.id)
      addToast('已取消收藏', 'success')
    } catch (e) {
      setFavorites(prev)
      addToast(errorMessage(e), 'error')
    }
  }

  if (favorites === null) {
    return (
      <div className="flex flex-col gap-1" role="status" aria-label="收藏加载中">
        {Array.from({ length: 5 }, (_, i) => (
          <div key={i} className="flex w-full items-center gap-3 px-2 py-2">
            <Skeleton className="size-10 shrink-0 rounded-md" />
            <div className="min-w-0 flex-1 space-y-1.5">
              <Skeleton className="h-4 w-2/3 max-w-52" />
              <Skeleton className="h-3 w-1/2 max-w-40" />
            </div>
          </div>
        ))}
      </div>
    )
  }

  const songFavs = favorites.filter((f) => f.song_id != null)
  const playlistFavs = favorites.filter((f) => f.playlist_id != null)
  const missingSongs = songFavs.filter((f) => !songData.has(f.song_id as number))

  if (favorites.length === 0) {
    return (
      <div className="flex flex-col items-center gap-2 px-4 py-12 text-center">
        <HeartFilled className="size-8 text-foreground-subtle" />
        <p className="text-sm font-medium text-foreground-muted">还没有收藏</p>
        <p className="text-xs text-foreground-subtle">在歌曲列表中点击心形按钮收藏喜欢的歌曲</p>
      </div>
    )
  }

  return (
    <div className="flex flex-col gap-5">
      {songFavs.length > 0 && (
        <section className="flex flex-col gap-2" aria-labelledby="fav-songs-title">
          <h2 id="fav-songs-title" className="text-sm font-semibold">
            歌曲收藏
          </h2>
          <ul className="flex flex-col gap-0.5" aria-label="收藏的歌曲">
            {songFavs.map((fav) => {
              const song = songData.get(fav.song_id as number)
              if (!song) return null
              return (
                <li key={fav.id}>
                  <SongRow song={song} favorited onToggleFavorite={() => void unfavorite(fav)} />
                </li>
              )
            })}
          </ul>
          {missingSongs.length > 0 && (
            <p className="text-xs text-foreground-subtle">部分歌曲信息加载中或已不可用…</p>
          )}
        </section>
      )}
      {playlistFavs.length > 0 && (
        <section className="flex flex-col gap-2" aria-labelledby="fav-playlists-title">
          <h2 id="fav-playlists-title" className="text-sm font-semibold">
            收藏的播放列表
          </h2>
          <ul className="flex flex-wrap gap-2" aria-label="收藏的播放列表">
            {playlistFavs.map((fav) => {
              const pl = playlists.find((p) => p.id === fav.playlist_id)
              return (
                <li key={fav.id}>
                  <span className="inline-flex max-w-full items-center gap-1.5 rounded-full border border-border bg-background-subtle py-1 pr-1 pl-3 text-sm">
                    <Bookmark className="size-3.5 shrink-0 text-foreground-subtle" />
                    <span className="max-w-40 truncate">{pl?.name ?? `播放列表 #${fav.playlist_id}`}</span>
                    <Button variant="ghost" size="icon-sm" aria-label="取消收藏" className="-me-0.5" onClick={() => void unfavorite(fav)}>
                      <X className="size-3.5" />
                    </Button>
                  </span>
                </li>
              )
            })}
          </ul>
        </section>
      )}
    </div>
  )
}
