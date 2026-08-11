import { useCallback, useEffect, useState } from 'react'
import { fetchAdminNcmStatus, importNcmPlaylist, saveAdminNcmCookie, startNcmImport, testAdminNcmLogin } from '@/api'
import type { ImportPlaylistResponse, NcmStatus } from '@/types'
import { useStore } from '@/store'
import { Badge } from '@appica/ui-react/badge'
import { Button } from '@appica/ui-react/button'
import { Field, FieldLabel, FieldDescription } from '@appica/ui-react/field'
import { Input } from '@appica/ui-react/input'
import { Skeleton } from '@appica/ui-react/skeleton'
import { Spinner } from '@appica/ui-react/spinner'
import { Textarea } from '@appica/ui-react/textarea'
import { CloudUpload, Link, ShieldCheck, Sparkle } from '@appica/icons-react'

function errMsg(e: unknown): string {
  return e instanceof Error ? e.message : '操作失败'
}

export function AdminNcm() {
  const addToast = useStore((s) => s.addToast)
  const [status, setStatus] = useState<NcmStatus | null>(null)
  const [loading, setLoading] = useState(true)
  const [cookie, setCookie] = useState('')
  const [saving, setSaving] = useState(false)
  const [testing, setTesting] = useState(false)
  const [link, setLink] = useState('')
  const [importing, setImporting] = useState(false)
  const [importResult, setImportResult] = useState<ImportPlaylistResponse | null>(null)
  const [importingBatch, setImportingBatch] = useState(false)

  const loadStatus = useCallback(async () => {
    try {
      const s = await fetchAdminNcmStatus()
      setStatus(s)
    } catch (e) {
      addToast(errMsg(e), 'error')
    } finally {
      setLoading(false)
    }
  }, [addToast])

  useEffect(() => {
    void loadStatus()
  }, [loadStatus])

  async function handleSaveCookie() {
    if (!cookie.trim()) {
      addToast('请输入网易云 Cookie', 'warning')
      return
    }
    setSaving(true)
    try {
      await saveAdminNcmCookie(cookie.trim())
      addToast('Cookie 已保存', 'success')
      await loadStatus()
    } catch (e) {
      addToast(errMsg(e), 'error')
    } finally {
      setSaving(false)
    }
  }

  async function handleTest() {
    setTesting(true)
    try {
      const result = await testAdminNcmLogin()
      if (!result.success) throw new Error(result.output || '网易云登录验证失败')
      addToast('网易云账号登录验证通过', 'success')
    } catch (e) {
      addToast(errMsg(e), 'error')
    } finally {
      setTesting(false)
    }
  }

  async function handleImport() {
    const l = link.trim()
    if (!l) {
      addToast('请输入歌单链接', 'warning')
      return
    }
    setImporting(true)
    try {
      const res = await importNcmPlaylist(l)
      setImportResult(res)
      addToast(`已解析歌单：${res.total} 首歌曲`, 'success')
    } catch (e) {
      addToast(errMsg(e), 'error')
    } finally {
      setImporting(false)
    }
  }

  async function handleStartImport() {
    if (!importResult) return
    setImportingBatch(true)
    try {
      await startNcmImport(importResult.batch_id)
      addToast('导入任务已开始', 'success')
    } catch (e) {
      addToast(errMsg(e), 'error')
    } finally {
      setImportingBatch(false)
    }
  }

  return (
    <div className="mx-auto w-full max-w-3xl space-y-8">
      <section aria-label="网易云 Cookie 设置" className="space-y-4">
        <div className="flex flex-wrap items-center gap-3">
          <h2 className="text-sm font-semibold">登录状态</h2>
          {loading ? (
            <Skeleton className="h-5 w-20 rounded-full" />
          ) : (
            <Badge variant={status?.configured ? 'success' : 'soft'} size="sm">
              {status?.configured ? '已配置' : '未配置'}
            </Badge>
          )}
          {status?.method && <span className="text-foreground-muted text-xs">方式：{status.method}</span>}
          {status?.phone_hint && <span className="text-foreground-muted text-xs">账号：{status.phone_hint}</span>}
        </div>
        <Field>
          <FieldLabel>网易云 Cookie</FieldLabel>
          <Textarea
            inputSize="sm"
            rows={5}
            placeholder={'粘贴 MUSIC_U / MUSIC_A 等网易云登录 Cookie'}
            value={cookie}
            onChange={(e) => setCookie(e.target.value)}
          />
          <FieldDescription>保存后可用于下载会员音质与导入歌单。</FieldDescription>
        </Field>
        <div className="flex flex-wrap gap-2">
          <Button variant="primary" size="sm" onClick={handleSaveCookie} disabled={saving}>
            {saving ? <Spinner currentColor className="size-4" aria-label="保存中" /> : <CloudUpload data-icon="start" />}
            保存 Cookie
          </Button>
          <Button variant="outline" size="sm" onClick={handleTest} disabled={testing}>
            {testing ? <Spinner currentColor className="size-4" aria-label="测试中" /> : <ShieldCheck data-icon="start" />}
            测试登录
          </Button>
        </div>
      </section>

      <section aria-label="导入播放列表" className="space-y-4">
        <h2 className="text-sm font-semibold">导入播放列表</h2>
        <Field>
          <FieldLabel>歌单链接</FieldLabel>
          <Input
            inputSize="sm"
            placeholder="例如 https://music.163.com/playlist?id=123456789"
            value={link}
            onChange={(e) => setLink(e.target.value)}
          />
        </Field>
        <Button variant="outline" size="sm" onClick={handleImport} disabled={importing}>
          {importing ? <Spinner currentColor className="size-4" aria-label="解析中" /> : <Link data-icon="start" />}
          解析歌单
        </Button>
        {importResult && (
          <div className="bg-background-subtle border-border-muted flex flex-wrap items-center gap-3 rounded-xl border p-4">
            <span className="text-sm">共解析到 <strong>{importResult.total}</strong> 首歌曲</span>
            <span className="text-foreground-muted text-xs">批次号：{importResult.batch_id}</span>
            {importResult.message && <span className="text-foreground-muted min-w-0 truncate text-xs">{importResult.message}</span>}
            <Button
              variant="primary"
              size="sm"
              className="ms-auto"
              onClick={handleStartImport}
              disabled={importingBatch}
            >
              {importingBatch ? <Spinner currentColor className="size-4" aria-label="导入中" /> : <Sparkle data-icon="start" />}
              开始导入
            </Button>
          </div>
        )}
      </section>
    </div>
  )
}
