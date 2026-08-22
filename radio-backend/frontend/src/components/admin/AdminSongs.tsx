import { useCallback, useEffect, useState } from 'react'
import { applyMetadataCandidate, deleteSongAdmin, downloadSongUrl, enrichSongMetadata, fetchAdminSongs, fetchMetadataCandidates, fetchMetadataJob, rescanSongs, updateSongMetadata, uploadSongFile } from '@/api'
import type { Song } from '@/types'
import type { MetadataCandidate, MetadataJob } from '@/types'
import { useStore } from '@/store'
import { AlertDialog, AlertDialogContent, AlertDialogHeader, AlertDialogTitle, AlertDialogDescription, AlertDialogFooter, AlertDialogClose } from '@appica/ui-react/alert-dialog'
import { Button } from '@appica/ui-react/button'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogBody, DialogFooter, DialogClose } from '@appica/ui-react/dialog'
import { Input } from '@appica/ui-react/input'
import { ScrollArea } from '@appica/ui-react/scroll-area'
import { Skeleton } from '@appica/ui-react/skeleton'
import { Spinner } from '@appica/ui-react/spinner'
import { Table, TableHeader, TableBody, TableRow, TableHead, TableCell } from '@appica/ui-react/table'
import { Edit, PlayerPlay, Refresh, Trash, Upload } from '@appica/icons-react'
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
  const [enriching, setEnriching] = useState(false)
  const [metadataJob, setMetadataJob] = useState<MetadataJob | null>(null)

  // Upload dialog
  const [uploadOpen, setUploadOpen] = useState(false)
  const [file, setFile] = useState<File | null>(null)
  const [uploading, setUploading] = useState(false)

  // Preview dialog
  const [preview, setPreview] = useState<Song | null>(null)

  // Delete confirmation
  const [deleteTarget, setDeleteTarget] = useState<Song | null>(null)
  const [deleting, setDeleting] = useState(false)
  const [editTarget, setEditTarget] = useState<Song | null>(null)
  const [editTitle, setEditTitle] = useState('')
  const [editArtist, setEditArtist] = useState('')
  const [editAlbum, setEditAlbum] = useState('')
  const [savingEdit, setSavingEdit] = useState(false)
  const [candidateTarget, setCandidateTarget] = useState<Song | null>(null)
  const [candidates, setCandidates] = useState<MetadataCandidate[]>([])
  const [applyingCandidate, setApplyingCandidate] = useState(false)

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

  useEffect(() => {
    if (!metadataJob || !['queued', 'running'].includes(metadataJob.status)) return
    const id = window.setInterval(() => {
      void fetchMetadataJob(metadataJob.id).then(setMetadataJob).catch(() => undefined)
    }, 1500)
    return () => window.clearInterval(id)
  }, [metadataJob])

  useEffect(() => {
    if (!candidateTarget) { setCandidates([]); return }
    void fetchMetadataCandidates(candidateTarget.id).then(setCandidates).catch(() => setCandidates([]))
  }, [candidateTarget])

  async function handleRescan() {
    setRescanning(true)
    try {
      const job = await rescanSongs()
      setMetadataJob(job)
      addToast(`已触发扫描任务：${job.id}`, 'success')
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

  async function handleEnrichMetadata() {
    setEnriching(true)
    try {
      const job = await enrichSongMetadata()
      setMetadataJob(job)
      addToast(`元数据任务已开始：${job.id}`, 'success')
      await loadSongs()
    } catch (e) {
      addToast(errMsg(e), 'error')
    } finally {
      setEnriching(false)
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

  function openEdit(song: Song) {
    setEditTarget(song)
    setEditTitle(song.title)
    setEditArtist(song.artist)
    setEditAlbum(song.album)
  }

  async function saveEdit() {
    if (!editTarget) return
    setSavingEdit(true)
    try {
      await updateSongMetadata(editTarget.id, { title: editTitle.trim(), artist: editArtist.trim(), album: editAlbum.trim() })
      addToast('元数据已保存并锁定', 'success')
      setEditTarget(null)
      await loadSongs()
    } catch (e) {
      addToast(errMsg(e), 'error')
    } finally {
      setSavingEdit(false)
    }
  }

  async function chooseCandidate(candidate: MetadataCandidate) {
    if (!candidateTarget) return
    setApplyingCandidate(true)
    try {
      await applyMetadataCandidate(candidateTarget.id, candidate)
      addToast('候选已应用', 'success')
      setCandidateTarget(null)
      await loadSongs()
    } catch (e) {
      addToast(errMsg(e), 'error')
    } finally {
      setApplyingCandidate(false)
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
        <Button variant="outline" size="sm" onClick={handleEnrichMetadata} disabled={enriching}>
          {enriching ? (
            <Spinner currentColor className="size-4" aria-label="补全中" />
          ) : (
            <Refresh data-icon="start" />
          )}
          补全元数据
        </Button>
        {!loading && (
          <span className="text-foreground-muted ms-auto text-xs">{songs.length} 首歌曲</span>
        )}
      </div>

      {metadataJob && (
        <div className="rounded-lg border border-border-subtle bg-background-subtle px-3 py-2 text-xs text-foreground-muted">
          <div className="flex items-center justify-between gap-3">
            <span>元数据任务 {metadataJob.status === 'completed' ? '已完成' : metadataJob.status === 'failed' ? '失败' : '处理中'}</span>
            <span className="tabular-nums">{metadataJob.processed}/{metadataJob.total} · 匹配 {metadataJob.matched} · 待确认 {metadataJob.needs_review} · 失败 {metadataJob.failed}</span>
          </div>
          {metadataJob.error && <p className="mt-1 text-error">{metadataJob.error}</p>}
        </div>
      )}

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
                <TableHead className="w-24">来源</TableHead>
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
                  <TableCell className="text-xs text-foreground-muted">{song.metadata_source || '—'}</TableCell>
                  <TableCell className="tabular-nums whitespace-nowrap">{formatTime(song.duration_ms)}</TableCell>
                  <TableCell className="tabular-nums whitespace-nowrap">{formatBytes(song.filesize)}</TableCell>
                  <TableCell>
                    <div className="flex items-center justify-end gap-1">
                      <Button
                        variant="ghost"
                        size="icon-sm"
                        aria-label={`编辑 ${song.title}`}
                        onClick={() => openEdit(song)}
                      >
                        <Edit />
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon-sm"
                        aria-label={`查看匹配候选 ${song.title}`}
                        onClick={() => setCandidateTarget(song)}
                      >
                        <Refresh />
                      </Button>
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

      <Dialog open={candidateTarget !== null} onOpenChange={(open) => { if (!open && !applyingCandidate) setCandidateTarget(null) }}>
        <DialogContent className="sm:w-140">
          <DialogHeader>
            <DialogTitle>在线匹配候选</DialogTitle>
            <DialogDescription>{candidateTarget ? `${candidateTarget.title} — ${candidateTarget.artist || '未知艺术家'}` : ''}</DialogDescription>
          </DialogHeader>
          <DialogBody className="max-h-[55vh] space-y-2 overflow-y-auto">
            {candidates.length === 0 ? <p className="py-6 text-center text-sm text-foreground-muted">暂无待确认候选，请先运行补全任务。</p> : candidates.map((candidate) => (
              <div key={`${candidate.provider}:${candidate.external_id}`} className="flex items-center gap-3 rounded-lg border border-border-subtle p-3">
                <div className="min-w-0 flex-1">
                  <p className="truncate text-sm font-medium">{candidate.title}</p>
                  <p className="truncate text-xs text-foreground-muted">{candidate.artists.join(', ')}{candidate.album ? ` · ${candidate.album}` : ''}</p>
                  <p className="text-xs text-foreground-subtle">{candidate.provider} · 分数 {candidate.score}</p>
                </div>
                <Button size="sm" onClick={() => void chooseCandidate(candidate)} disabled={applyingCandidate}>采用</Button>
              </div>
            ))}
          </DialogBody>
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
              <audio controls src={downloadSongUrl(preview.id)} preload="none" className="w-full" />
            )}
          </DialogBody>
        </DialogContent>
      </Dialog>

      <Dialog open={editTarget !== null} onOpenChange={(open) => { if (!open && !savingEdit) setEditTarget(null) }}>
        <DialogContent className="sm:w-120">
          <DialogHeader>
            <DialogTitle>编辑元数据</DialogTitle>
            <DialogDescription>保存后这些字段会锁定，不会被自动匹配覆盖。</DialogDescription>
          </DialogHeader>
          <DialogBody className="space-y-3">
            <Input value={editTitle} onChange={(e) => setEditTitle(e.target.value)} placeholder="标题" aria-label="标题" />
            <Input value={editArtist} onChange={(e) => setEditArtist(e.target.value)} placeholder="艺术家" aria-label="艺术家" />
            <Input value={editAlbum} onChange={(e) => setEditAlbum(e.target.value)} placeholder="专辑" aria-label="专辑" />
          </DialogBody>
          <DialogFooter>
            <DialogClose render={<Button variant="secondary" disabled={savingEdit}>取消</Button>} />
            <Button onClick={saveEdit} disabled={savingEdit || !editTitle.trim()}>
              {savingEdit ? <Spinner currentColor className="size-4" aria-label="保存中" /> : null}
              保存并锁定
            </Button>
          </DialogFooter>
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
