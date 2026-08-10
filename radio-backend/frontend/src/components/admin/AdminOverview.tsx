import { useEffect, useState } from 'react'
import { fetchAdminLogs, fetchAdminStats, fetchHistory } from '@/api'
import type { AdminLogEntry, AdminStats, HistoryItem } from '@/types'
import { useStore } from '@/store'
import { ScrollArea } from '@appica/ui-react/scroll-area'
import { Skeleton } from '@appica/ui-react/skeleton'
import { Table, TableHeader, TableBody, TableRow, TableHead, TableCell } from '@appica/ui-react/table'
import { formatDateTime } from '@/lib/format'

/** Chinese labels for known stats keys; unknown keys fall back to the raw key. */
const STAT_LABELS: Record<string, string> = {
  song_count: '歌曲总数',
  user_count: '用户总数',
  queue_size: '队列待播',
  queue_pending: '待播放',
  queue_played: '已播放',
  listener_count: '当前听众',
  total_duration_ms: '曲库总时长(ms)',
  total_filesize: '曲库总大小(B)',
  request_count: '点播次数',
  play_count: '播放次数',
}

function statsCards(stats: AdminStats | null): { key: string; label: string; value: number }[] {
  if (!stats) return []
  const numeric = Object.entries(stats).filter(([, v]) => typeof v === 'number') as [string, number][]
  const known = numeric.filter(([k]) => ['song_count', 'user_count', 'queue_size'].includes(k))
  const rest = numeric.filter(([k]) => !['song_count', 'user_count', 'queue_size'].includes(k))
  return [...known, ...rest].map(([key, value]) => ({ key, label: STAT_LABELS[key] ?? key, value }))
}

export function AdminOverview() {
  const addToast = useStore((s) => s.addToast)
  const [stats, setStats] = useState<AdminStats | null>(null)
  const [logs, setLogs] = useState<AdminLogEntry[]>([])
  const [history, setHistory] = useState<HistoryItem[]>([])
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    let cancelled = false
    Promise.all([fetchAdminStats(), fetchAdminLogs(), fetchHistory()])
      .then(([s, l, h]) => {
        if (cancelled) return
        setStats(s)
        setLogs(Array.isArray(l) ? l : [])
        setHistory(Array.isArray(h) ? h : [])
      })
      .catch((e) => {
        if (!cancelled) addToast(e instanceof Error ? e.message : '加载失败', 'error')
      })
      .finally(() => {
        if (!cancelled) setLoading(false)
      })
    return () => {
      cancelled = true
    }
  }, [addToast])

  const cards = statsCards(stats)

  return (
    <div className="space-y-8">
      <section aria-labelledby="admin-stats-title">
        <h2 id="admin-stats-title" className="text-foreground-muted mb-3 text-sm font-semibold">
          统计
        </h2>
        {loading ? (
          <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-4">
            {Array.from({ length: 4 }).map((_, i) => (
              <Skeleton key={i} className="h-20 rounded-xl" />
            ))}
          </div>
        ) : cards.length === 0 ? (
          <p className="text-foreground-muted text-sm">暂无统计数据</p>
        ) : (
          <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-4">
            {cards.map((card) => (
              <div
                key={card.key}
                className="bg-background-subtle border-border-muted flex min-w-0 flex-col gap-1 rounded-xl border p-4"
              >
                <span className="text-foreground-muted truncate text-xs">{card.label}</span>
                <span className="tabular-nums text-2xl font-semibold">{card.value.toLocaleString()}</span>
              </div>
            ))}
          </div>
        )}
      </section>

      <section aria-labelledby="admin-history-title">
        <h2 id="admin-history-title" className="text-foreground-muted mb-3 text-sm font-semibold">
          播放历史
        </h2>
        {loading ? (
          <div className="space-y-2">
            {Array.from({ length: 4 }).map((_, i) => (
              <Skeleton key={i} className="h-9 w-full rounded-lg" />
            ))}
          </div>
        ) : history.length === 0 ? (
          <p className="text-foreground-muted text-sm">暂无播放历史</p>
        ) : (
          <ScrollArea orientation="horizontal" scrollShadow className="w-full">
            <Table size="sm" hoverableRows className="w-full min-w-[420px]">
              <TableHeader>
                <TableRow>
                  <TableHead className="w-32">歌曲</TableHead>
                  <TableHead className="w-36">播放时间</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {history.map((item) => (
                  <TableRow key={item.id}>
                    <TableCell className="font-medium whitespace-nowrap">歌曲 #{item.song_id}</TableCell>
                    <TableCell className="text-foreground-muted whitespace-nowrap">
                      {formatDateTime(item.played_at)}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </ScrollArea>
        )}
      </section>

      <section aria-labelledby="admin-logs-title">
        <h2 id="admin-logs-title" className="text-foreground-muted mb-3 text-sm font-semibold">
          管理日志
        </h2>
        {loading ? (
          <div className="space-y-2">
            {Array.from({ length: 4 }).map((_, i) => (
              <Skeleton key={i} className="h-9 w-full rounded-lg" />
            ))}
          </div>
        ) : logs.length === 0 ? (
          <p className="text-foreground-muted text-sm">暂无管理日志</p>
        ) : (
          <ScrollArea orientation="horizontal" scrollShadow className="w-full">
            <Table size="sm" hoverableRows className="w-full min-w-[560px]">
              <TableHeader>
                <TableRow>
                  <TableHead className="w-32">操作</TableHead>
                  <TableHead>详情</TableHead>
                  <TableHead className="w-36">时间</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {logs.map((log) => (
                  <TableRow key={log.id}>
                    <TableCell className="font-medium whitespace-nowrap">{log.action}</TableCell>
                    <TableCell className="max-w-[340px] truncate" title={log.details}>
                      {log.details || '—'}
                    </TableCell>
                    <TableCell className="text-foreground-muted whitespace-nowrap">
                      {formatDateTime(log.created_at)}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </ScrollArea>
        )}
      </section>
    </div>
  )
}
