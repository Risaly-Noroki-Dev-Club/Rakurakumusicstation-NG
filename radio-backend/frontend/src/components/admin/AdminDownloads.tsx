import { useEffect, useState } from 'react'
import { fetchBatchDownloadStatus, startBatchDownload } from '@/api'
import type { BatchDownloadStatus } from '@/types'
import { useStore } from '@/store'
import { Button } from '@appica/ui-react/button'
import { Field, FieldLabel, FieldDescription } from '@appica/ui-react/field'
import { Input } from '@appica/ui-react/input'
import { Progress, ProgressLabel, ProgressValue } from '@appica/ui-react/progress'
import { Select, SelectTrigger, SelectValue, SelectContent, SelectItem } from '@appica/ui-react/select'
import { Spinner } from '@appica/ui-react/spinner'
import { Textarea } from '@appica/ui-react/textarea'
import { CircleCheckFilled, CircleXFilled, Download, Rotate360 } from '@appica/icons-react'

const POLL_INTERVAL_MS = 2000
const MAX_DURATION_MS = 10 * 60 * 1000

const SOURCE_OPTIONS = [
  { value: 'ncm', label: '网易云音乐' },
  { value: 'netdisk', label: '网盘' },
  { value: 'spotify', label: 'Spotify' },
]

const LYRICS_OPTIONS = [
  { value: 'none', label: '不保存歌词' },
  { value: 'separate', label: '保存为单独文件' },
  { value: 'overwrite', label: '覆盖原歌词' },
]

function errMsg(e: unknown): string {
  return e instanceof Error ? e.message : '操作失败'
}

export function AdminDownloads() {
  const addToast = useStore((s) => s.addToast)
  const [source, setSource] = useState('ncm')
  const [lyricsMode, setLyricsMode] = useState('none')
  const [quality, setQuality] = useState('')
  const [itemsText, setItemsText] = useState('')
  const [starting, setStarting] = useState(false)
  const [taskId, setTaskId] = useState<string | null>(null)
  const [status, setStatus] = useState<BatchDownloadStatus | null>(null)
  const [done, setDone] = useState(false)

  async function start() {
    const urls = itemsText.split('\n').map((s) => s.trim()).filter(Boolean)
    if (urls.length === 0) {
      addToast('请至少输入一条下载链接', 'warning')
      return
    }
    setStarting(true)
    try {
      const res = await startBatchDownload({
        source,
        items: urls.map((url) => ({ url })),
        lyrics_save_mode: lyricsMode,
        ...(quality.trim() ? { quality: quality.trim() } : {}),
      })
      setTaskId(res.task_id)
      setStatus(null)
      setDone(false)
    } catch (e) {
      addToast(errMsg(e), 'error')
    } finally {
      setStarting(false)
    }
  }

  // Poll the batch task until it finishes (max ~10 min); cleaned up on unmount.
  useEffect(() => {
    if (!taskId) return
    let cancelled = false
    const startedAt = Date.now()
    const timer = window.setInterval(() => {
      void (async () => {
        if (cancelled) return
        try {
          const st = await fetchBatchDownloadStatus(taskId)
          if (cancelled) return
          setStatus(st)
          if (!st.running) {
            window.clearInterval(timer)
            setDone(true)
            addToast(`下载完成：成功 ${st.success}，失败 ${st.failed}`, st.failed > 0 ? 'warning' : 'success')
          } else if (Date.now() - startedAt > MAX_DURATION_MS) {
            window.clearInterval(timer)
            setDone(true)
            addToast('下载超时，请稍后到后台检查结果', 'warning')
          }
        } catch (e) {
          if (cancelled) return
          window.clearInterval(timer)
          setDone(true)
          addToast(errMsg(e), 'error')
        }
      })()
    }, POLL_INTERVAL_MS)
    return () => {
      cancelled = true
      window.clearInterval(timer)
    }
  }, [taskId, addToast])

  async function restart() {
    setTaskId(null)
    setStatus(null)
    setDone(false)
    await start()
  }

  const total = status?.total ?? 0
  const processed = status ? status.success + status.failed : 0
  const percent = total > 0 ? Math.min(100, Math.round((processed / total) * 100)) : 0
  const running = taskId !== null && !done

  return (
    <div className="mx-auto w-full max-w-3xl space-y-8">
      <section className="space-y-4" aria-label="批量下载设置">
        <div className="grid gap-4 sm:grid-cols-2">
          <Field>
            <FieldLabel>下载来源</FieldLabel>
            <Select
              value={source}
              onValueChange={(value) => { if (typeof value === 'string') setSource(value) }}
            >
              <SelectTrigger>
                <SelectValue placeholder="选择来源" />
              </SelectTrigger>
              <SelectContent>
                {SOURCE_OPTIONS.map((opt) => (
                  <SelectItem key={opt.value} value={opt.value}>
                    {opt.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </Field>
          <Field>
            <FieldLabel>歌词保存模式</FieldLabel>
            <Select
              value={lyricsMode}
              onValueChange={(value) => { if (typeof value === 'string') setLyricsMode(value) }}
            >
              <SelectTrigger>
                <SelectValue placeholder="选择歌词保存方式" />
              </SelectTrigger>
              <SelectContent>
                {LYRICS_OPTIONS.map((opt) => (
                  <SelectItem key={opt.value} value={opt.value}>
                    {opt.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </Field>
        </div>
        <Field>
          <FieldLabel>音质（可选）</FieldLabel>
          <Input
            inputSize="sm"
            placeholder="例如 320k / flac / 128k"
            value={quality}
            onChange={(e) => setQuality(e.target.value)}
          />
          <FieldDescription>留空则使用默认音质。</FieldDescription>
        </Field>
        <Field>
          <FieldLabel>下载链接</FieldLabel>
          <Textarea
            inputSize="sm"
            rows={6}
            placeholder={'每行一条链接，例如：\nhttps://music.163.com/song?id=123456\nhttps://example.com/file.mp3'}
            value={itemsText}
            onChange={(e) => setItemsText(e.target.value)}
          />
          <FieldDescription>每行一条链接，支持网易云 / 网盘 / Spotify 链接。</FieldDescription>
        </Field>
        <Button variant="primary" size="md" onClick={start} disabled={starting || running}>
          {starting ? <Spinner currentColor className="size-4" aria-label="提交中" /> : <Download data-icon="start" />}
          {running ? '下载中…' : '开始下载'}
        </Button>
      </section>

      {taskId !== null && (
        <section aria-label="下载进度" className="space-y-3">
          <Progress value={percent} variant="bar" className="w-full">
            <ProgressLabel className="text-foreground-muted">下载进度</ProgressLabel>
            <ProgressValue className="text-foreground-muted tabular-nums">{() => `${processed} / ${total}`}</ProgressValue>
          </Progress>
          <p className="text-foreground-muted text-xs">
            成功 {status?.success ?? 0} · 失败 {status?.failed ?? 0}
            {done && total > 0 && ` · 进度 ${percent}%`}
          </p>
          {status && status.items.length > 0 && (
            <ul className="border-border-muted divide-border-muted divide-y rounded-xl border">
              {status.items.map((item, i) => (
                <li key={i} className="flex min-w-0 items-start gap-2 px-3 py-2">
                  {item.success ? (
                    <CircleCheckFilled className="text-success mt-0.5 size-4 shrink-0" aria-hidden />
                  ) : (
                    <CircleXFilled className="text-error mt-0.5 size-4 shrink-0" aria-hidden />
                  )}
                  <div className="min-w-0 flex-1">
                    <p className="min-w-0 truncate text-sm">
                      {item.title || item.artist || item.file_path || `项目 ${i + 1}`}
                    </p>
                    {!item.success && item.error && (
                      <p className="text-error-muted min-w-0 truncate text-xs" title={item.error}>
                        {item.error}
                      </p>
                    )}
                  </div>
                </li>
              ))}
            </ul>
          )}
          {done && (
            <Button variant="outline" size="sm" onClick={restart} disabled={starting}>
              <Rotate360 data-icon="start" />
              重新开始
            </Button>
          )}
        </section>
      )}
    </div>
  )
}
