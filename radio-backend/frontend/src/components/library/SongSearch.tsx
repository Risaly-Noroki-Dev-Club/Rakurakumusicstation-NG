import { useCallback, useEffect, useState } from 'react'
import { Input } from '@appica/ui-react/input'
import { Button } from '@appica/ui-react/button'
import { Skeleton } from '@appica/ui-react/skeleton'
import { Spinner } from '@appica/ui-react/spinner'
import { Music, Search } from '@appica/icons-react'
import { addFavorite, fetchFavorites, removeFavorite, searchSongs } from '@/api'
import { SongRow } from '@/components/library/SongRow'
import { useStore, type Toast } from '@/store'
import type { Favorite, SongSummary } from '@/types'

const PAGE_SIZE = 50

function buildFavMap(favorites: Favorite[]): Map<number, number> {
  const map = new Map<number, number>()
  for (const f of favorites) {
    if (f.song_id != null) map.set(f.song_id, f.id)
  }
  return map
}

function errorMessage(e: unknown): string {
  return e instanceof Error ? e.message : '操作失败'
}

/** 歌曲 tab：防抖搜索 + 分页加载 + 收藏状态。 */
export function SongSearch() {
  // store.ts is FINAL; its Toast level union omits 'success' but Toasts.tsx
  // renders a success variant — widen the binding here so call sites stay clean.
  const addToast = useStore((s) => s.addToast) as (
    message: string,
    level?: Toast['level'] | 'success',
  ) => void
  const [query, setQuery] = useState('')
  const [debounced, setDebounced] = useState('')
  const [songs, setSongs] = useState<SongSummary[]>([])
  const [total, setTotal] = useState(0)
  const [loading, setLoading] = useState(true)
  const [loadingMore, setLoadingMore] = useState(false)
  const [favBySong, setFavBySong] = useState<Map<number, number>>(new Map())

  // 300ms 防抖。
  useEffect(() => {
    const t = setTimeout(() => setDebounced(query.trim()), 300)
    return () => clearTimeout(t)
  }, [query])

  // 收藏状态（heart 图标）。
  useEffect(() => {
    let cancelled = false
    fetchFavorites()
      .then((favs) => {
        if (!cancelled) setFavBySong(buildFavMap(favs))
      })
      .catch(() => undefined)
    return () => {
      cancelled = true
    }
  }, [])

  // 搜索（挂载时以空查询抓取全部歌曲，之后每次防抖后重新搜索）。
  useEffect(() => {
    let cancelled = false
    setLoading(true)
    searchSongs(debounced, PAGE_SIZE, 0)
      .then((res) => {
        if (cancelled) return
        setSongs(res.data)
        setTotal(res.total)
      })
      .catch((e) => {
        if (cancelled) return
        setSongs([])
        setTotal(0)
        addToast(errorMessage(e), 'error')
      })
      .finally(() => {
        if (!cancelled) setLoading(false)
      })
    return () => {
      cancelled = true
    }
  }, [debounced, addToast])

  const loadMore = useCallback(async () => {
    if (loadingMore) return
    setLoadingMore(true)
    try {
      const res = await searchSongs(debounced, PAGE_SIZE, songs.length)
      setSongs((prev) => [...prev, ...res.data])
      setTotal(res.total)
    } catch (e) {
      addToast(errorMessage(e), 'error')
    } finally {
      setLoadingMore(false)
    }
  }, [debounced, songs.length, loadingMore, addToast])

  const toggleFavorite = useCallback(
    async (songId: number) => {
      const prev = favBySong
      const favId = prev.get(songId)
      const next = new Map(prev)
      if (favId != null) next.delete(songId)
      else next.set(songId, -1) // 占位，成功后再从服务端取回真实 id
      setFavBySong(next)
      try {
        if (favId != null) await removeFavorite(favId)
        else await addFavorite(songId)
        const fresh = await fetchFavorites()
        setFavBySong(buildFavMap(fresh))
      } catch (e) {
        setFavBySong(prev)
        addToast(errorMessage(e), 'error')
      }
    },
    [favBySong, addToast],
  )

  return (
    <div className="flex flex-col gap-3">
      <Input
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder="搜索歌曲、歌手或专辑"
        startSlot={<Search className="size-4.5" />}
        clearable
        onClear={() => setQuery('')}
        aria-label="搜索歌曲"
        inputSize="md"
      />
      {loading ? (
        <div className="flex flex-col gap-1" role="status" aria-label="歌曲加载中">
          {Array.from({ length: 8 }, (_, i) => (
            <div key={i} className="flex w-full items-center gap-3 px-2 py-2">
              <Skeleton className="size-10 shrink-0 rounded-md" />
              <div className="min-w-0 flex-1 space-y-1.5">
                <Skeleton className="h-4 w-2/3 max-w-52" />
                <Skeleton className="h-3 w-1/2 max-w-40" />
              </div>
              <Skeleton className="h-3 w-8 shrink-0" />
            </div>
          ))}
        </div>
      ) : songs.length === 0 ? (
        <div className="flex flex-col items-center gap-2 px-4 py-12 text-center">
          <Music className="size-8 text-foreground-subtle" />
          <p className="text-sm font-medium text-foreground-muted">
            {debounced ? `未找到与“${debounced}”相关的歌曲` : '曲库中暂无歌曲'}
          </p>
        </div>
      ) : (
        <>
          <p className="text-xs text-foreground-subtle" role="status">
            共 {total} 首
          </p>
          <ul className="flex flex-col gap-0.5" aria-label="歌曲搜索结果">
            {songs.map((song) => (
              <li key={song.id}>
                <SongRow
                  song={song}
                  favorited={favBySong.has(song.id)}
                  onToggleFavorite={(id) => void toggleFavorite(id)}
                />
              </li>
            ))}
          </ul>
          {songs.length < total && (
            <Button variant="outline" className="mt-1 w-full" disabled={loadingMore} onClick={() => void loadMore()}>
              {loadingMore ? <Spinner className="size-4.5" currentColor /> : null}
              {loadingMore ? '加载中…' : '加载更多'}
            </Button>
          )}
        </>
      )}
    </div>
  )
}
