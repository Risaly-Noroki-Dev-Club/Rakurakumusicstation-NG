import { useEffect, useState } from 'react'
import { fetchStationSettings, saveStationSettings, uploadStationIcon } from '@/api'
import type { StationSettings } from '@/types'
import { useStore } from '@/store'
import { Button } from '@appica/ui-react/button'
import { Field, FieldLabel, FieldDescription } from '@appica/ui-react/field'
import { Input } from '@appica/ui-react/input'
import { Skeleton } from '@appica/ui-react/skeleton'
import { Spinner } from '@appica/ui-react/spinner'
import { Textarea } from '@appica/ui-react/textarea'
import { Thumbnail } from '@appica/ui-react/thumbnail'
import { Check, Radio, Upload } from '@appica/icons-react'

const MAX_ICON_BYTES = 2 * 1024 * 1024

function errMsg(e: unknown): string {
  return e instanceof Error ? e.message : '操作失败'
}

export function AdminStationSettings() {
  const addToast = useStore((s) => s.addToast)
  const [settings, setSettings] = useState<StationSettings | null>(null)
  const [loading, setLoading] = useState(true)
  const [stationName, setStationName] = useState('')
  const [shortName, setShortName] = useState('')
  const [subtitle, setSubtitle] = useState('')
  const [description, setDescription] = useState('')
  const [saving, setSaving] = useState(false)
  const [iconFile, setIconFile] = useState<File | null>(null)
  const [uploadingIcon, setUploadingIcon] = useState(false)
  const [iconFailed, setIconFailed] = useState(false)

  async function load() {
    try {
      const s = await fetchStationSettings()
      setSettings(s)
      setStationName(s.station_name ?? '')
      setShortName(s.short_name ?? '')
      setSubtitle(s.subtitle ?? '')
      setDescription(s.description ?? '')
    } catch (e) {
      addToast(errMsg(e), 'error')
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    void load()
  }, [])

  // resolved_icon_url may be relative (or absent) — render as-is; fall back when it fails to load.
  const iconSrc = settings?.resolved_icon_url || settings?.icon_url || ''

  useEffect(() => {
    setIconFailed(false)
  }, [iconSrc])

  async function handleSave() {
    setSaving(true)
    try {
      await saveStationSettings({
        station_name: stationName.trim(),
        short_name: shortName.trim(),
        subtitle: subtitle.trim(),
        description: description.trim(),
      })
      addToast('已保存（部分设置需重启服务端生效）', 'success')
    } catch (e) {
      addToast(errMsg(e), 'error')
    } finally {
      setSaving(false)
    }
  }

  async function handleUploadIcon() {
    if (!iconFile) {
      addToast('请先选择图标文件', 'warning')
      return
    }
    if (iconFile.size > MAX_ICON_BYTES) {
      addToast('图标文件不能超过 2MB', 'warning')
      return
    }
    setUploadingIcon(true)
    try {
      await uploadStationIcon(iconFile)
      addToast('图标已上传', 'success')
      setIconFile(null)
      await load()
    } catch (e) {
      addToast(errMsg(e), 'error')
    } finally {
      setUploadingIcon(false)
    }
  }

  return (
    <div className="mx-auto w-full max-w-3xl space-y-8">
      {loading ? (
        <div className="space-y-4">
          <Skeleton className="h-10 w-full rounded-lg" />
          <Skeleton className="h-10 w-full rounded-lg" />
          <Skeleton className="h-10 w-full rounded-lg" />
          <Skeleton className="h-28 w-full rounded-lg" />
        </div>
      ) : (
        <>
          <section aria-label="电台信息" className="space-y-4">
            <h2 className="text-sm font-semibold">电台信息</h2>
            <Field>
              <FieldLabel>电台名称</FieldLabel>
              <Input
                inputSize="sm"
                placeholder="Rakuraku Music Station"
                value={stationName}
                onChange={(e) => setStationName(e.target.value)}
              />
            </Field>
            <div className="grid gap-4 sm:grid-cols-2">
              <Field>
                <FieldLabel>短名称</FieldLabel>
                <Input
                  inputSize="sm"
                  placeholder="Rakuraku"
                  value={shortName}
                  onChange={(e) => setShortName(e.target.value)}
                />
              </Field>
              <Field>
                <FieldLabel>副标题</FieldLabel>
                <Input
                  inputSize="sm"
                  placeholder="轻松听，慢慢点"
                  value={subtitle}
                  onChange={(e) => setSubtitle(e.target.value)}
                />
              </Field>
            </div>
            <Field>
              <FieldLabel>电台简介</FieldLabel>
              <Textarea
                inputSize="sm"
                rows={4}
                placeholder="向听众介绍你的电台……"
                value={description}
                onChange={(e) => setDescription(e.target.value)}
              />
            </Field>
            <Button variant="primary" size="sm" onClick={handleSave} disabled={saving}>
              {saving ? <Spinner currentColor className="size-4" aria-label="保存中" /> : <Check data-icon="start" />}
              保存设置
            </Button>
          </section>

          <section aria-label="电台图标" className="space-y-4">
            <h2 className="text-sm font-semibold">电台图标</h2>
            <div className="flex flex-wrap items-center gap-4">
              <Thumbnail
                size="lg"
                shape="rounded"
                variant="image"
                src={iconSrc || undefined}
                alt="电台图标"
                onLoadingStatusChange={(s) => {
                  if (s === 'error') setIconFailed(true)
                }}
                className="shrink-0"
              >
                {(!iconSrc || iconFailed) && <Radio className="size-5" aria-hidden />}
              </Thumbnail>
              <div className="min-w-0 flex-1 space-y-2">
                <Input
                  type="file"
                  accept="image/*"
                  inputSize="sm"
                  aria-label="选择图标文件"
                  onChange={(e) => setIconFile(e.target.files?.[0] ?? null)}
                  className="file:me-2 file:rounded-md file:border-0 file:bg-primary file:px-3 file:py-1 file:text-xs file:font-medium file:text-primary-foreground"
                />
                <FieldDescription>支持 PNG / JPG / SVG，不超过 2MB。</FieldDescription>
                <Button variant="outline" size="sm" onClick={handleUploadIcon} disabled={uploadingIcon || !iconFile}>
                  {uploadingIcon ? <Spinner currentColor className="size-4" aria-label="上传中" /> : <Upload data-icon="start" />}
                  上传图标
                </Button>
              </div>
            </div>
          </section>
        </>
      )}
    </div>
  )
}
