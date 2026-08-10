import { useEffect, useMemo, useState } from 'react'
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@appica/ui-react/tabs'
import { Skeleton } from '@appica/ui-react/skeleton'
import { Thumbnail } from '@appica/ui-react/thumbnail'
import { History, Lock } from '@appica/icons-react'
import { fetchHistory } from '@/api'
import { useStore } from '@/store'
import { QueueList } from '@/components/queue/QueueList'
import { formatDateTime } from '@/lib/format'

export default function UpNextPage() {
  const isAdmin = useStore((s) => s.auth?.role === 'admin')
  const history = useStore((s) => s.history)
  const [historyLoading, setHistoryLoading] = useState(false)
  const [historyFailed, setHistoryFailed] = useState(false)

  // History is admin-only data; non-admins get 403 from the endpoint.
  useEffect(() => {
    if (!isAdmin) return
    let cancelled = false
    setHistoryLoading(true)
    setHistoryFailed(false)
    fetchHistory()
      .then((h) => {
        if (!cancelled) useStore.getState().setHistory(h)
      })
      .catch(() => {
        if (!cancelled) setHistoryFailed(true)
      })
      .finally(() => {
        if (!cancelled) setHistoryLoading(false)
      })
    return () => {
      cancelled = true
    }
  }, [isAdmin])

  // History items carry no song summary — sort and render honestly (歌曲 #id).
  const sortedHistory = useMemo(
    () => [...history].sort((a, b) => new Date(b.played_at).getTime() - new Date(a.played_at).getTime()),
    [history],
  )

  return (
    <div className="mx-auto w-full max-w-6xl px-4 py-6 sm:py-8">
      <h1 className="text-foreground-intense mb-4 text-xl font-semibold sm:text-2xl">接下来</h1>

      <Tabs defaultValue="up-next" variant="line">
        <TabsList>
          <TabsTrigger value="up-next">即将播放</TabsTrigger>
          <TabsTrigger value="history">播放历史</TabsTrigger>
        </TabsList>

        <TabsContent value="up-next" className="pt-4">
          <QueueList />
        </TabsContent>

        <TabsContent value="history" className="pt-4">
          {!isAdmin ? (
            <div className="flex flex-col items-center gap-2 rounded-xl border border-dashed border-border-muted bg-background-subtle/40 px-6 py-14 text-center">
              <Lock className="text-foreground-muted size-10" aria-hidden="true" />
              <p className="text-foreground-muted text-sm">仅管理员可见</p>
            </div>
          ) : historyLoading ? (
            <ul
              aria-label="播放历史"
              className="divide-border-muted divide-y overflow-hidden rounded-xl border border-border-muted bg-background"
            >
              {Array.from({ length: 5 }, (_, i) => (
                <li key={i} className="flex items-center gap-3 px-3 py-3 sm:px-4">
                  <Skeleton className="size-9 shrink-0 rounded-lg" />
                  <div className="min-w-0 flex-1 space-y-2">
                    <Skeleton className="h-3.5 w-1/2 rounded" />
                    <Skeleton className="h-3 w-1/4 rounded" />
                  </div>
                </li>
              ))}
            </ul>
          ) : historyFailed ? (
            <div className="flex flex-col items-center gap-2 rounded-xl border border-dashed border-border-muted bg-background-subtle/40 px-6 py-14 text-center">
              <p className="text-foreground-muted text-sm">加载播放历史失败</p>
            </div>
          ) : sortedHistory.length === 0 ? (
            <div className="flex flex-col items-center gap-2 rounded-xl border border-dashed border-border-muted bg-background-subtle/40 px-6 py-14 text-center">
              <History className="text-foreground-muted size-10" aria-hidden="true" />
              <p className="text-foreground-muted text-sm">暂无播放历史</p>
            </div>
          ) : (
            <ul
              aria-label="播放历史"
              className="divide-border-muted divide-y overflow-hidden rounded-xl border border-border-muted bg-background"
            >
              {sortedHistory.map((h) => (
                <li key={h.id} className="flex items-center gap-3 px-3 py-3 sm:px-4">
                  <Thumbnail size="sm" shape="rounded" variant="icon-soft" className="shrink-0" aria-hidden="true">
                    <History />
                  </Thumbnail>
                  <div className="min-w-0 flex-1">
                    <p className="text-foreground-intense truncate text-sm font-medium">歌曲 #{h.song_id}</p>
                  </div>
                  <span className="text-foreground-subtle shrink-0 text-xs tabular-nums">{formatDateTime(h.played_at)}</span>
                </li>
              ))}
            </ul>
          )}
        </TabsContent>
      </Tabs>
    </div>
  )
}
