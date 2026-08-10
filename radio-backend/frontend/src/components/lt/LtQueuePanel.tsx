import { useEffect } from 'react'
import { Badge } from '@appica/ui-react/badge'
import { Playlist } from '@appica/icons-react'
import { fetchQueue } from '@/api'
import { useStore } from '@/store'
import { formatDateTime } from '@/lib/format'

/** 一起听 — shared request queue panel. */
export function LtQueuePanel() {
  const queue = useStore((s) => s.queue)

  useEffect(() => {
    let cancelled = false
    void fetchQueue()
      .then((q) => {
        if (!cancelled) useStore.getState().setQueue(q)
      })
      .catch(() => undefined)
    return () => {
      cancelled = true
    }
  }, [])

  return (
    <aside className="border-border-muted flex min-h-0 flex-col border-t p-4 md:w-80 md:border-t-0 md:border-l">
      <div className="flex shrink-0 items-center gap-2 pb-3">
        <Playlist data-icon="start" className="text-primary" />
        <h2 className="text-foreground-intense font-semibold">点歌队列</h2>
        <Badge variant="secondary" aria-label={`队列中共 ${queue.length} 首`}>
          {queue.length}
        </Badge>
      </div>
      {queue.length === 0 ? (
        <p className="text-foreground-muted text-sm">队列空空的，去曲库点一首吧</p>
      ) : (
        <ol aria-label="点歌队列" className="flex min-h-0 flex-col gap-1 overflow-y-auto">
          {queue.map((item, index) => (
            <li
              key={item.id}
              className="border-border-muted flex items-center gap-3 rounded-lg border p-2.5"
            >
              <span className="text-foreground-subtle w-5 shrink-0 text-center text-sm tabular-nums">{index + 1}</span>
              <div className="min-w-0 flex-1">
                <p className="text-foreground-intense truncate text-sm font-medium">
                  {item.song?.title ?? '未知歌曲'}
                </p>
                <p className="text-foreground-muted truncate text-xs">
                  {item.song?.artist ?? item.requested_by ?? '匿名'}
                  {item.song && ` · ${formatDateTime(item.added_at)}`}
                </p>
              </div>
            </li>
          ))}
        </ol>
      )}
    </aside>
  )
}
