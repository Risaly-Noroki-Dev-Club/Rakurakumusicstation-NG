import { useCallback, useEffect, useState } from 'react'
import { Button } from '@appica/ui-react/button'
import { Badge } from '@appica/ui-react/badge'
import { Skeleton } from '@appica/ui-react/skeleton'
import { Spinner } from '@appica/ui-react/spinner'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogBody,
  DialogFooter,
  DialogClose,
} from '@appica/ui-react/dialog'
import {
  AlertDialog,
  AlertDialogContent,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogClose,
} from '@appica/ui-react/alert-dialog'
import { Field, FieldLabel } from '@appica/ui-react/field'
import { Input } from '@appica/ui-react/input'
import { Switch } from '@appica/ui-react/switch'
import { FolderOpen, PlayerPlay, Plus, Trash } from '@appica/icons-react'
import {
  addToQueue,
  createPlaylist,
  deletePlaylist,
  fetchPlaylistDetail,
  fetchPlaylists,
  removeSongFromPlaylist,
} from '@/api'
import { formatDateTime, formatTime } from '@/lib/format'
import { useStore, type Toast } from '@/store'
import type { PlaylistDetail, PlaylistWithCount, SongSummary } from '@/types'

function errorMessage(e: unknown): string {
  return e instanceof Error ? e.message : '操作失败'
}

/** 播放列表 tab：卡片网格 + 详情/新建/删除。 */
export function PlaylistsTab() {
  // store.ts is FINAL; its Toast level union omits 'success' but Toasts.tsx
  // renders a success variant — widen the binding here so call sites stay clean.
  const addToast = useStore((s) => s.addToast) as (
    message: string,
    level?: Toast['level'] | 'success',
  ) => void
  const [playlists, setPlaylists] = useState<PlaylistWithCount[] | null>(null)
  const [detail, setDetail] = useState<PlaylistDetail | null>(null)
  const [detailId, setDetailId] = useState<number | null>(null)
  const [detailLoading, setDetailLoading] = useState(false)
  const [removingId, setRemovingId] = useState<number | null>(null)
  const [queueingId, setQueueingId] = useState<number | null>(null)
  const [createOpen, setCreateOpen] = useState(false)
  const [name, setName] = useState('')
  const [isPublic, setIsPublic] = useState(false)
  const [creating, setCreating] = useState(false)
  const [deleteTarget, setDeleteTarget] = useState<PlaylistWithCount | null>(null)
  const [deleting, setDeleting] = useState(false)

  const refresh = useCallback(async () => {
    try {
      setPlaylists(await fetchPlaylists())
    } catch (e) {
      addToast(errorMessage(e), 'error')
    }
  }, [addToast])

  useEffect(() => {
    let cancelled = false
    fetchPlaylists()
      .then((list) => {
        if (!cancelled) setPlaylists(list)
      })
      .catch((e) => {
        if (!cancelled) addToast(errorMessage(e), 'error')
      })
    return () => {
      cancelled = true
    }
  }, [addToast])

  // 打开卡片时拉取详情。
  useEffect(() => {
    if (detailId == null) return
    let cancelled = false
    setDetailLoading(true)
    fetchPlaylistDetail(detailId)
      .then((d) => {
        if (!cancelled) setDetail(d)
      })
      .catch((e) => {
        if (cancelled) return
        addToast(errorMessage(e), 'error')
        setDetailId(null)
      })
      .finally(() => {
        if (!cancelled) setDetailLoading(false)
      })
    return () => {
      cancelled = true
    }
  }, [detailId, addToast])

  const openDetail = (id: number) => {
    setDetail(null)
    setDetailId(id)
  }

  const handleCreate = async () => {
    const trimmed = name.trim()
    if (!trimmed || creating) return
    setCreating(true)
    try {
      await createPlaylist(trimmed, isPublic)
      addToast('已创建播放列表', 'success')
      setName('')
      setIsPublic(false)
      setCreateOpen(false)
      void refresh()
    } catch (e) {
      addToast(errorMessage(e), 'error')
    } finally {
      setCreating(false)
    }
  }

  const handleDelete = async () => {
    if (!deleteTarget || deleting) return
    setDeleting(true)
    try {
      await deletePlaylist(deleteTarget.id)
      addToast('已删除播放列表', 'success')
      setDeleteTarget(null)
      if (detailId === deleteTarget.id) setDetailId(null)
      void refresh()
    } catch (e) {
      addToast(errorMessage(e), 'error')
    } finally {
      setDeleting(false)
    }
  }

  const handleRemoveSong = async (songId: number) => {
    if (detailId == null || removingId !== null) return
    setRemovingId(songId)
    try {
      await removeSongFromPlaylist(detailId, songId)
      addToast('已从播放列表移除', 'success')
      setDetail(await fetchPlaylistDetail(detailId))
      void refresh()
    } catch (e) {
      addToast(errorMessage(e), 'error')
    } finally {
      setRemovingId(null)
    }
  }

  const handleQueue = async (song: SongSummary) => {
    if (queueingId !== null) return
    setQueueingId(song.id)
    try {
      await addToQueue(song.id)
      addToast('已加入点歌队列', 'success')
    } catch (e) {
      addToast(errorMessage(e), 'error')
    } finally {
      setQueueingId(null)
    }
  }

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center justify-between gap-3">
        <p className="text-xs text-foreground-subtle" role="status">
          {playlists ? `共 ${playlists.length} 个播放列表` : ''}
        </p>
        <Button size="sm" onClick={() => setCreateOpen(true)}>
          <Plus data-icon="start" />
          新建播放列表
        </Button>
      </div>

      {playlists === null ? (
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3" role="status" aria-label="播放列表加载中">
          {Array.from({ length: 3 }, (_, i) => (
            <Skeleton key={i} className="h-28 rounded-lg" />
          ))}
        </div>
      ) : playlists.length === 0 ? (
        <div className="flex flex-col items-center gap-2 px-4 py-12 text-center">
          <FolderOpen className="size-8 text-foreground-subtle" />
          <p className="text-sm font-medium text-foreground-muted">还没有播放列表</p>
          <p className="text-xs text-foreground-subtle">点击右上角按钮创建一个吧</p>
        </div>
      ) : (
        <ul className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3" aria-label="播放列表">
          {playlists.map((p) => (
            <li key={p.id} className="min-w-0">
              <div
                role="button"
                tabIndex={0}
                aria-label={`打开播放列表 ${p.name}`}
                onClick={() => openDetail(p.id)}
                onKeyDown={(e) => {
                  if (e.target === e.currentTarget && (e.key === 'Enter' || e.key === ' ')) {
                    e.preventDefault()
                    openDetail(p.id)
                  }
                }}
                className="flex h-full min-w-0 cursor-pointer flex-col gap-2 rounded-lg border border-border bg-background-subtle/50 p-3 transition-colors hover:bg-background-subtle focus-visible:outline-ring"
              >
                <div className="flex min-w-0 items-center gap-2">
                  <FolderOpen className="size-5 shrink-0 text-primary" />
                  <p className="min-w-0 flex-1 truncate text-sm font-medium">{p.name}</p>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    aria-label={`删除播放列表 ${p.name}`}
                    className="-me-1 -mr-1 shrink-0"
                    onClick={(e) => {
                      e.stopPropagation()
                      setDeleteTarget(p)
                    }}
                  >
                    <Trash />
                  </Button>
                </div>
                <div className="flex min-w-0 items-center gap-2 text-xs text-foreground-muted">
                  <Badge variant="soft" size="xs" className="shrink-0">
                    {p.song_count} 首
                  </Badge>
                  {p.is_public && (
                    <Badge variant="info" size="xs" className="shrink-0">
                      公开
                    </Badge>
                  )}
                  <span className="min-w-0 truncate">{formatDateTime(p.created_at)}</span>
                </div>
              </div>
            </li>
          ))}
        </ul>
      )}

      {/* 详情 */}
      <Dialog open={detailId !== null} onOpenChange={(open) => !open && setDetailId(null)}>
        <DialogContent className="sm:w-130">
          <DialogHeader>
            <DialogTitle className="min-w-0 truncate">{detail?.playlist.name ?? ''}</DialogTitle>
            <DialogDescription>
              {detail ? `${detail.songs.length} 首歌曲` : '加载中…'}
            </DialogDescription>
          </DialogHeader>
          <DialogBody className="flex max-h-[60vh] flex-col gap-1 overflow-y-auto">
            {detailLoading || detail === null ? (
              <div className="flex flex-col gap-2" role="status" aria-label="播放列表歌曲加载中">
                {Array.from({ length: 5 }, (_, i) => (
                  <div key={i} className="flex w-full items-center gap-3 px-2 py-2">
                    <Skeleton className="size-8 shrink-0 rounded-md" />
                    <div className="min-w-0 flex-1 space-y-1.5">
                      <Skeleton className="h-4 w-2/3 max-w-48" />
                      <Skeleton className="h-3 w-1/2 max-w-36" />
                    </div>
                  </div>
                ))}
              </div>
            ) : detail.songs.length === 0 ? (
              <div className="flex flex-col items-center gap-2 px-4 py-10 text-center">
                <FolderOpen className="size-7 text-foreground-subtle" />
                <p className="text-sm text-foreground-muted">播放列表为空，去歌曲页添加吧</p>
              </div>
            ) : (
              <ul className="flex flex-col gap-0.5" aria-label="播放列表歌曲">
                {detail.songs.map((song) => (
                  <li key={song.id} className="flex min-w-0 items-center gap-2.5 rounded-md px-2 py-1.5 hover:bg-background-subtle">
                    <div className="min-w-0 flex-1">
                      <p className="min-w-0 truncate text-sm">{song.title}</p>
                      <p className="min-w-0 truncate text-xs text-foreground-muted">{song.artist}</p>
                    </div>
                    <span className="shrink-0 text-xs tabular-nums text-foreground-subtle">
                      {formatTime(song.duration_ms)}
                    </span>
                    <Button
                      variant="ghost"
                      size="icon-sm"
                      aria-label={`点歌：${song.title}`}
                      disabled={queueingId === song.id}
                      onClick={() => void handleQueue(song)}
                    >
                      {queueingId === song.id ? <Spinner className="size-4" currentColor /> : <PlayerPlay />}
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon-sm"
                      aria-label={`从播放列表移除：${song.title}`}
                      disabled={removingId === song.id}
                      onClick={() => void handleRemoveSong(song.id)}
                    >
                      {removingId === song.id ? <Spinner className="size-4" currentColor /> : <Trash className="text-error" />}
                    </Button>
                  </li>
                ))}
              </ul>
            )}
          </DialogBody>
        </DialogContent>
      </Dialog>

      {/* 新建 */}
      <Dialog open={createOpen} onOpenChange={setCreateOpen}>
        <DialogContent className="sm:w-100">
          <DialogHeader>
            <DialogTitle>新建播放列表</DialogTitle>
            <DialogDescription>为收藏的歌曲创建一个新播放列表</DialogDescription>
          </DialogHeader>
          <DialogBody className="flex flex-col gap-4">
            <Field>
              <FieldLabel>名称</FieldLabel>
              <Input
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="播放列表名称"
                aria-label="播放列表名称"
                onKeyDown={(e) => {
                  if (e.key === 'Enter') void handleCreate()
                }}
              />
            </Field>
            <div className="flex items-center justify-between gap-3">
              <div className="min-w-0">
                <p className="text-sm font-medium">公开播放列表</p>
                <p className="text-xs text-foreground-muted">其他听众可以看到并点播这个列表</p>
              </div>
              <Switch checked={isPublic} onCheckedChange={setIsPublic} aria-label="公开播放列表" />
            </div>
          </DialogBody>
          <DialogFooter>
            <DialogClose render={<Button variant="outline" disabled={creating} />}>
              取消
            </DialogClose>
            <Button onClick={() => void handleCreate()} disabled={!name.trim() || creating}>
              {creating ? <Spinner className="size-4.5" currentColor /> : null}
              {creating ? '创建中…' : '创建'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* 删除确认 */}
      <AlertDialog
        open={deleteTarget !== null}
        onOpenChange={(open) => {
          if (!open && !deleting) setDeleteTarget(null)
        }}
      >
        <AlertDialogContent className="sm:w-100">
          <AlertDialogHeader>
            <AlertDialogTitle>删除播放列表？</AlertDialogTitle>
            <AlertDialogDescription>
              将删除“{deleteTarget?.name}”及其中的全部歌曲，此操作无法撤销。
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogClose render={<Button variant="outline" disabled={deleting} />}>
              取消
            </AlertDialogClose>
            <Button variant="destructive" onClick={() => void handleDelete()} disabled={deleting}>
              {deleting ? <Spinner className="size-4.5" currentColor /> : null}
              {deleting ? '删除中…' : '删除'}
            </Button>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  )
}
