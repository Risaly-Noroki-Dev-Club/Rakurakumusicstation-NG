import { useEffect, useState } from 'react'
import {
  AlertDialog,
  AlertDialogContent,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogClose,
} from '@appica/ui-react/alert-dialog'
import { Badge } from '@appica/ui-react/badge'
import { Button } from '@appica/ui-react/button'
import { Loader, Lock, PlayerSkipForward, Refresh } from '@appica/icons-react'
import { fetchQueue, skipCurrent } from '@/api'
import { useStore, type Toast } from '@/store'
import { QueueList } from '@/components/queue/QueueList'

export default function QueuePage() {
  const queue = useStore((s) => s.queue)
  const isAdmin = useStore((s) => s.auth?.role === 'admin')
  const [refreshing, setRefreshing] = useState(false)
  const [skipOpen, setSkipOpen] = useState(false)
  const [skipping, setSkipping] = useState(false)

  // Prime the store on mount; WS queue_update keeps it fresh afterwards.
  useEffect(() => {
    let cancelled = false
    fetchQueue()
      .then((q) => {
        if (!cancelled) useStore.getState().setQueue(q)
      })
      .catch(() => undefined)
    return () => {
      cancelled = true
    }
  }, [])

  const handleRefresh = async () => {
    setRefreshing(true)
    try {
      const q = await fetchQueue()
      useStore.getState().setQueue(q)
    } catch (err) {
      useStore.getState().addToast(err instanceof Error ? err.message : '刷新队列失败', 'error')
    } finally {
      setRefreshing(false)
    }
  }

  const handleSkip = async () => {
    setSkipping(true)
    try {
      await skipCurrent()
      useStore.getState().addToast('已跳过当前歌曲', 'success' as Toast['level'])
      setSkipOpen(false)
    } catch (err) {
      useStore.getState().addToast(err instanceof Error ? err.message : '跳过失败', 'error')
      setSkipOpen(false)
    } finally {
      setSkipping(false)
    }
  }

  return (
    <div className="mx-auto w-full max-w-6xl px-4 py-6 sm:py-8">
      <div className="mb-4 flex flex-wrap items-center gap-2">
        <h1 className="text-foreground-intense text-xl font-semibold sm:text-2xl">点歌队列</h1>
        <Badge variant="soft" size="sm" aria-label={`队列中共 ${queue.length} 首`}>
          {queue.length}
        </Badge>
        <div className="ms-auto flex items-center gap-2">
          {isAdmin && (
            <Button variant="outline" size="sm" onClick={() => setSkipOpen(true)}>
              <PlayerSkipForward data-icon="start" />
              跳过当前
            </Button>
          )}
          <Button variant="ghost" size="icon-sm" aria-label="刷新队列" onClick={() => void handleRefresh()} disabled={refreshing}>
            {refreshing ? <Loader className="animate-spin" /> : <Refresh />}
          </Button>
        </div>
      </div>

      {!isAdmin && (
        <p className="text-foreground-muted mb-4 flex items-center gap-1.5 text-sm">
          <Lock className="size-3.5" aria-hidden="true" />
          跳过当前歌曲仅管理员可用
        </p>
      )}

      <QueueList />

      <AlertDialog open={skipOpen} onOpenChange={setSkipOpen}>
        <AlertDialogContent className="sm:w-110">
          <AlertDialogHeader>
            <AlertDialogTitle>跳过当前歌曲？</AlertDialogTitle>
            <AlertDialogDescription>将立即结束正在播放的歌曲，并播放队列中的下一首。</AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogClose render={<Button variant="outline" disabled={skipping}>取消</Button>} />
            <Button variant="destructive" onClick={handleSkip} disabled={skipping}>
              {skipping && <Loader data-icon="start" className="animate-spin" />}
              跳过
            </Button>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  )
}
