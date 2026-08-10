import { useEffect, useState } from 'react'
import { Link } from 'react-router-dom'
import {
  AlertDialog,
  AlertDialogTrigger,
  AlertDialogContent,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogClose,
} from '@appica/ui-react/alert-dialog'
import { Badge } from '@appica/ui-react/badge'
import { Button } from '@appica/ui-react/button'
import { Skeleton } from '@appica/ui-react/skeleton'
import { ChevronDown, ChevronUp, Library, Loader, Music, Trash } from '@appica/icons-react'
import { fetchQueue, moveQueueItem, removeQueueItem, coverUrl } from '@/api'
import { SongArtwork } from '@/components/SongArtwork'
import { loadSongIndex, resolveSong } from '@/lib/songIndex'
import { useStore, type Toast } from '@/store'
import type { QueueItemDisplay } from '@/types'
import { formatDateTime } from '@/lib/format'

export interface QueueListProps {
  /** Override the data source; defaults to the store's queue. */
  items?: QueueItemDisplay[]
  /** Called after any mutation-driven refetch settles. */
  onChanged?: () => void
}

const STATUS_META: Record<QueueItemDisplay['status'], { label: string; variant: 'soft' | 'primary' | 'light' }> = {
  pending: { label: '等待中', variant: 'soft' },
  playing: { label: '播放中', variant: 'primary' },
  played: { label: '已播放', variant: 'light' },
  skipped: { label: '已跳过', variant: 'light' },
}

function itemTitle(item: QueueItemDisplay): string {
  return item.song?.title ?? '未知歌曲'
}

/** Trash button + AlertDialog confirm; closes only after the request settles. */
function RemoveItemButton({ item, onRemoved }: { item: QueueItemDisplay; onRemoved: () => void }) {
  const [open, setOpen] = useState(false)
  const [busy, setBusy] = useState(false)

  const confirm = async () => {
    setBusy(true)
    try {
      // NOTE: backend quirk — item.song.id is always 0; use item.id.
      await removeQueueItem(item.id)
      useStore.getState().addToast('已从队列移除', 'success' as Toast['level'])
      onRemoved()
    } catch (err) {
      useStore.getState().addToast(err instanceof Error ? err.message : '移除失败', 'error')
    } finally {
      setBusy(false)
      setOpen(false)
    }
  }

  return (
    <AlertDialog open={open} onOpenChange={setOpen}>
      <AlertDialogTrigger render={<Button variant="ghost" size="icon-sm" aria-label={`移除 ${itemTitle(item)}`} />}>
        <Trash />
      </AlertDialogTrigger>
      <AlertDialogContent className="sm:w-110">
        <AlertDialogHeader>
          <AlertDialogTitle>移除歌曲？</AlertDialogTitle>
          <AlertDialogDescription>确定要将「{itemTitle(item)}」从队列中移除吗？</AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogClose render={<Button variant="outline" disabled={busy}>取消</Button>} />
          <Button variant="destructive" onClick={confirm} disabled={busy}>
            {busy ? <Loader data-icon="start" className="animate-spin" /> : <Trash data-icon="start" />}
            移除
          </Button>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  )
}

/**
 * Queue list. Renders the store's queue (or an explicit `items` override)
 * with admin remove/move actions; mutations refetch and push into the store.
 */
export function QueueList({ items, onChanged }: QueueListProps) {
  const storeQueue = useStore((s) => s.queue)
  const isAdmin = useStore((s) => s.auth?.role === 'admin')
  const queue = items ?? storeQueue
  const [loaded, setLoaded] = useState(false)
  const [busyId, setBusyId] = useState<number | null>(null)
  const [indexReady, setIndexReady] = useState(false)

  // Build the title|artist → real SongSummary index so queue rows (whose
  // embedded song.id is hardcoded to 0 by the backend) can load covers.
  useEffect(() => {
    let cancelled = false
    loadSongIndex()
      .then(() => {
        if (!cancelled) setIndexReady(true)
      })
      .catch(() => undefined)
    return () => {
      cancelled = true
    }
  }, [])

  // First load (store mode only — an `items` prop implies the owner fetches).
  useEffect(() => {
    if (items) {
      setLoaded(true)
      return
    }
    let cancelled = false
    fetchQueue()
      .then((q) => {
        if (!cancelled) useStore.getState().setQueue(q)
      })
      .catch(() => undefined)
      .finally(() => {
        if (!cancelled) setLoaded(true)
      })
    return () => {
      cancelled = true
    }
  }, [items])

  const refresh = () => {
    void fetchQueue()
      .then((q) => useStore.getState().setQueue(q))
      .catch(() => useStore.getState().addToast('刷新队列失败', 'error'))
      .finally(() => onChanged?.())
  }

  const handleMove = async (item: QueueItemDisplay, delta: number) => {
    setBusyId(item.id)
    try {
      await moveQueueItem(item.id, item.position + delta)
      useStore.getState().addToast('已调整顺序', 'success' as Toast['level'])
      refresh()
    } catch (err) {
      useStore.getState().addToast(err instanceof Error ? err.message : '调整顺序失败', 'error')
    } finally {
      setBusyId(null)
    }
  }

  if (!loaded && queue.length === 0) {
    return (
      <ul
        aria-label="点歌队列"
        className="divide-border-muted divide-y overflow-hidden rounded-xl border border-border-muted bg-background"
      >
        {Array.from({ length: 4 }, (_, i) => (
          <li key={i} className="flex items-center gap-3 px-3 py-3 sm:px-4">
            <Skeleton className="size-4 shrink-0 rounded" />
            <Skeleton className="size-9 shrink-0 rounded-lg" />
            <div className="min-w-0 flex-1 space-y-2">
              <Skeleton className="h-3.5 w-2/3 rounded" />
              <Skeleton className="h-3 w-1/3 rounded" />
            </div>
            <Skeleton className="h-5 w-16 shrink-0 rounded-full" />
          </li>
        ))}
      </ul>
    )
  }

  if (queue.length === 0) {
    return (
      <div className="flex flex-col items-center gap-3 rounded-xl border border-dashed border-border-muted bg-background-subtle/40 px-6 py-14 text-center">
        <Music className="text-foreground-muted size-10" aria-hidden="true" />
        <p className="text-foreground-muted text-sm">队列空，去曲库点歌吧</p>
        <Button render={<Link to="/library" />} variant="primary" size="sm">
          <Library data-icon="start" />
          去曲库点歌
        </Button>
      </div>
    )
  }

  return (
    <ul
      aria-label="点歌队列"
      className="divide-border-muted divide-y overflow-hidden rounded-xl border border-border-muted bg-background"
    >
      {queue.map((item, index) => {
        const meta = STATUS_META[item.status] ?? STATUS_META.pending
        const secondary = [item.song?.artist, item.requested_by].filter(Boolean).join(' · ')
        const realSong = indexReady ? resolveSong(item.song) : null
        const artworkSong = realSong ?? item.song
        return (
          <li key={item.id} className="flex items-center gap-3 px-3 py-3 sm:px-4">
            <span className="text-foreground-muted w-6 shrink-0 text-center text-xs font-medium tabular-nums">
              {item.position}
            </span>
            <SongArtwork
              hasCover={artworkSong?.has_cover ?? false}
              coverSrc={realSong && realSong.id > 0 ? coverUrl(realSong.id) : undefined}
              size="sm"
              className="shrink-0"
            />
            <div className="min-w-0 flex-1">
              <p className="text-foreground-intense truncate text-sm font-medium">{itemTitle(item)}</p>
              {secondary && <p className="text-foreground-muted truncate text-xs">{secondary}</p>}
              <p className="text-foreground-subtle truncate text-xs">{formatDateTime(item.added_at)}</p>
            </div>
            <Badge variant={meta.variant} size="sm" className="shrink-0">
              {meta.label}
            </Badge>
            {isAdmin && (
              <div className="flex shrink-0 items-center gap-0.5">
                <Button
                  variant="ghost"
                  size="icon-sm"
                  aria-label={`上移 ${itemTitle(item)}`}
                  disabled={busyId !== null || index === 0}
                  onClick={() => void handleMove(item, -1)}
                >
                  <ChevronUp />
                </Button>
                <Button
                  variant="ghost"
                  size="icon-sm"
                  aria-label={`下移 ${itemTitle(item)}`}
                  disabled={busyId !== null || index === queue.length - 1}
                  onClick={() => void handleMove(item, 1)}
                >
                  <ChevronDown />
                </Button>
                <RemoveItemButton item={item} onRemoved={refresh} />
              </div>
            )}
          </li>
        )
      })}
    </ul>
  )
}
