import { useCallback, useEffect, useState } from 'react'
import { deleteSongAdmin, fetchAdminSongs, rescanSongs, songStreamUrl, uploadSongFile } from '@/api'
import type { Song } from '@/types'
import { useStore } from '@/store'
import { AlertDialog, AlertDialogContent, AlertDialogHeader, AlertDialogTitle, AlertDialogDescription, AlertDialogFooter, AlertDialogClose } from '@appica/ui-react/alert-dialog'
import { Button } from '@appica/ui-react/button'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogBody, DialogFooter, DialogClose } from '@appica/ui-react/dialog'
import { Input } from '@appica/ui-react/input'
import { ScrollArea } from '@appica/ui-react/scroll-area'
import { Skeleton } from '@appica/ui-react/skeleton'
import { Spinner } from '@appica/ui-react/spinner'
import { Table, TableHeader, TableBody, TableRow, TableHead, TableCell } from '@appica/ui-react/table'
import { PlayerPlay, Refresh, Trash, Upload } from '@appica/icons-react'
import { formatBytes, formatTime } from '@/lib/format'

const MAX_UPLOAD_BYTES = 100 * 1024 * 1024

function errMsg(e: unknown): string {
  return e instanceof Error ? e.message : '操作失败'
}

export function AdminSongs() {
  const addToast = useStore((s) => s.addToast)
  const [songs, setSongs] = useState<Song[]>([])
  const [loading, setLoading] = useState(true)
  const [rescanning, setRescanning] = useState(false)

  // Upload dialog
  const [uploadOpen, setUploadOpen] = useState(false)
  const [file, setFile] = useState<File | null>(null)
  const [uploading, setUploading] = useState(false)

  // Preview dialog
  const [preview, setPreview] = useState<Song | null>(null)

  // Delete confirmation
  const [deleteTarget, setDeleteTarget] = useState<Song | null>(null)
  const [deleting, setDeleting] = useState(false)

  const loadSongs = useCallback(async () => {
    try {
      const list = await fetchAdminSongs()
      setSongs(Array.isArray(list) ? list : [])
    } catch (e) {
      addToast(errMsg(e), 'error')
    } finally {
      setLoading(false)
    }
  }, [addToast])

  useEffect(() => {
    void loadSongs()
  }, [loadSongs])

  async function handleRescan() {
    setRescanning(true)
    try {
      await rescanSongs()
      addToast('已触发扫描', 'success')
    } catch (e) {
      addToast(errMsg(e), 'error')
    } finally {
      setRescanning(false)
    }
  }

  async function handleUpload() {
    if (!file) {
      addToast('请先选择音频文件', 'warning')
      return
    }
    if (file.size > MAX_UPLOAD_BYTES) {
      addToast('文件超过 100MB 上限', 'warning')
      return
    }
    setUploading(true)
    try {
      await uploadSongFile(file)
      addToast('上传成功', 'success')
      setUploadOpen(false)
      setFile(null)
      await loadSongs()
    } catch (e) {
      addToast(errMsg(e), 'error')
    } finally {
      setUploading(false)
    }
  }

  async function confirmDelete() {
    if (!deleteTarget) return
    setDeleting(true)
    try {
      await deleteSongAdmin(deleteTarget.id)
      addToast('歌曲已删除', 'success')
      setDeleteTarget(null)
      await loadSongs()
    } catch (e) {
      addToast(errMsg(e), 'error')
    } finally {
      setDeleting(false)
    }
  }

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center gap-2">
        <Button variant="outline" size="sm" onClick={handleRescan} disabled={rescanning}>
          {rescanning ? (
            <Spinner currentColor className="size-4" aria-label="扫描中" />
          ) : (
            <Refresh data-icon="start" />
          )}
          重新扫描
        </Button>
        <Button variant="primary" size="sm" onClick={() => setUploadOpen(true)}>
          <Upload data-icon="start" />
          上传
        </Button>
        {!loading && (
          <span className="text-foreground-muted ms-auto text-xs">{songs.length} 首歌曲</span>
        )}
      </div>

      {loading ? (
        <div className="space-y-2">
          {Array.from({ length: 6 }).map((_, i) => (
            <Skeleton key={i} className="h-10 w-full rounded-lg" />
          ))}
        </div>
      ) : songs.length === 0 ? (
        <p className="text-foreground-muted py-8 text-center text-sm">曲库为空，点击“重新扫描”或“上传”添加歌曲</p>
      ) : (
        <ScrollArea orientation="horizontal" scrollShadow className="w-full">
          <Table size="sm" stripedRows hoverableRows className="w-full min-w-[720px]">
            <TableHeader>
              <TableRow>
                <TableHead>标题</TableHead>
                <TableHead>艺术家</TableHead>
                <TableHead>专辑</TableHead>
                <TableHead className="w-20">时长</TableHead>
                <TableHead className="w-24">大小</TableHead>
                <TableHead className="w-24 text-right">操作</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {songs.map((song) => (
                <TableRow key={song.id}>
                  <TableCell className="max-w-[240px] truncate font-medium" title={song.title}>
                    {song.title}
                  </TableCell>
                  <TableCell className="max-w-[180px] truncate" title={song.artist}>
                    {song.artist || '—'}
                  </TableCell>
                  <TableCell className="max-w-[180px] truncate" title={song.album}>
                    {song.album || '—'}
                  </TableCell>
                  <TableCell className="tabular-nums whitespace-nowrap">{formatTime(song.duration_ms)}</TableCell>
                  <TableCell className="tabular-nums whitespace-nowrap">{formatBytes(song.filesize)}</TableCell>
                  <TableCell>
                    <div className="flex items-center justify-end gap-1">
                      <Button
                        variant="ghost"
                        size="icon-sm"
                        aria-label={`试听 ${song.title}`}
                        onClick={() => setPreview(song)}
                      >
                        <PlayerPlay />
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon-sm"
                        aria-label={`删除 ${song.title}`}
                        onClick={() => setDeleteTarget(song)}
                      >
                        <Trash />
                      </Button>
                    </div>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </ScrollArea>
      )}

      {/* Upload dialog */}
      <Dialog open={uploadOpen} onOpenChange={(open) => setUploadOpen(open)}>
        <DialogContent className="sm:w-110">
          <DialogHeader>
            <DialogTitle>上传歌曲</DialogTitle>
            <DialogDescription>选择本地音频文件上传到曲库（不超过 100MB）。</DialogDescription>
          </DialogHeader>
          <DialogBody className="space-y-3">
            <Input
              type="file"
              accept="audio/*"
              inputSize="sm"
              aria-label="选择音频文件"
              onChange={(e) => setFile(e.target.files?.[0] ?? null)}
              className="file:me-2 file:rounded-md file:border-0 file:bg-primary file:px-3 file:py-1 file:text-xs file:font-medium file:text-primary-foreground"
            />
            {file && (
              <p className="text-foreground-muted min-w-0 truncate text-xs">
                {file.name}（{formatBytes(file.size)}）
              </p>
            )}
          </DialogBody>
          <DialogFooter>
            <DialogClose render={<Button variant="secondary" disabled={uploading}>取消</Button>} />
            <Button onClick={handleUpload} disabled={uploading || !file}>
              {uploading ? <Spinner currentColor className="size-4" aria-label="上传中" /> : <Upload data-icon="start" />}
              上传
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Preview dialog */}
      <Dialog open={preview !== null} onOpenChange={(open) => { if (!open) setPreview(null) }}>
        <DialogContent className="sm:w-120">
          <DialogHeader>
            <DialogTitle>试听</DialogTitle>
            <DialogDescription>
              {preview ? `${preview.title}${preview.artist ? ` — ${preview.artist}` : ''}` : ''}
            </DialogDescription>
          </DialogHeader>
          <DialogBody>
            {preview && (
              <audio controls src={songStreamUrl(preview.id)} preload="none" className="w-full" />
            )}
          </DialogBody>
        </DialogContent>
      </Dialog>

      {/* Delete confirmation */}
      <AlertDialog
        open={deleteTarget !== null}
        onOpenChange={(open) => { if (!open && !deleting) setDeleteTarget(null) }}
      >
        <AlertDialogContent className="sm:w-100">
          <AlertDialogHeader>
            <AlertDialogTitle>删除歌曲</AlertDialogTitle>
            <AlertDialogDescription>
              确定要删除「{deleteTarget?.title ?? ''}」吗？此操作不可撤销。
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogClose render={<Button variant="secondary" disabled={deleting}>取消</Button>} />
            <Button variant="destructive" onClick={confirmDelete} disabled={deleting}>
              {deleting ? <Spinner currentColor className="size-4" aria-label="删除中" /> : <Trash data-icon="start" />}
              删除
            </Button>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  )
}
