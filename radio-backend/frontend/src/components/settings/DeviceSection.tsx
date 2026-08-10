import { useState, type FormEvent } from 'react'
import { Link, useNavigate } from 'react-router-dom'
import {
  Alert,
  AlertDescription,
  AlertIcon,
  AlertTitle,
} from '@appica/ui-react/alert'
import {
  AlertDialog,
  AlertDialogClose,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@appica/ui-react/alert-dialog'
import { Badge } from '@appica/ui-react/badge'
import { Button } from '@appica/ui-react/button'
import { Field, FieldDescription, FieldLabel } from '@appica/ui-react/field'
import { Input } from '@appica/ui-react/input'
import { Spinner } from '@appica/ui-react/spinner'
import {
  AlertTriangleFilled,
  Check,
  DeviceMobile,
  ExternalLink,
  InfoCircleFilled,
  Key,
  Logout,
  ShieldCheckFilled,
  UserFilled,
} from '@appica/icons-react'
import { adminLogout, claimAdmin, fetchMe, fetchStation, setDisplayName } from '@/api'
import { useStore } from '@/store'

function toast(message: string, level: 'info' | 'success' | 'warning' | 'error' = 'info') {
  useStore.getState().addToast(message, level)
}

function errMsg(err: unknown): string {
  return err instanceof Error && err.message ? err.message : '操作失败，请重试'
}

/** 首次部署：持有管理员令牌时可激活管理员身份。 */
function ClaimAdminBlock() {
  const [token, setToken] = useState('')
  const [claiming, setClaiming] = useState(false)

  const submit = async (e: FormEvent) => {
    e.preventDefault()
    const trimmed = token.trim()
    if (!trimmed || claiming) return
    setClaiming(true)
    try {
      await claimAdmin(trimmed)
      const [me, st] = await Promise.all([fetchMe(), fetchStation()])
      useStore.getState().setAuth(me)
      useStore.getState().setStation(st)
      setToken('')
      toast('管理员身份已激活', 'success')
    } catch (err) {
      toast(errMsg(err), 'error')
    } finally {
      setClaiming(false)
    }
  }

  return (
    <div className="flex flex-col gap-3">
      <Alert variant="warning">
        <AlertIcon>
          <AlertTriangleFilled />
        </AlertIcon>
        <AlertTitle>首次部署：需要管理员令牌</AlertTitle>
        <AlertDescription>电台尚未配置管理员。持有部署时生成的管理员令牌即可完成初始化。</AlertDescription>
      </Alert>
      <form onSubmit={submit} className="flex flex-col gap-2 sm:flex-row sm:items-center">
        <Input
          type="password"
          value={token}
          onChange={(e) => setToken(e.target.value)}
          placeholder="管理员令牌"
          aria-label="管理员令牌"
          autoComplete="off"
          className="min-w-0 flex-1"
        />
        <Button type="submit" variant="secondary" disabled={!token.trim() || claiming} className="shrink-0">
          {claiming ? <Spinner currentColor /> : <Key data-icon="start" />}
          {claiming ? '验证中…' : '申请管理员'}
        </Button>
      </form>
    </div>
  )
}

/** 尚未设置设备身份时，提示命名（setDisplayName 会创建设备）。 */
function DeviceNameForm() {
  const [name, setName] = useState('')
  const [saving, setSaving] = useState(false)

  const submit = async (e: FormEvent) => {
    e.preventDefault()
    const trimmed = name.trim()
    if (!trimmed || saving) return
    setSaving(true)
    try {
      await setDisplayName(trimmed)
      const me = await fetchMe()
      useStore.getState().setAuth(me)
      toast('设备名称已设置', 'success')
    } catch (err) {
      toast(errMsg(err), 'error')
    } finally {
      setSaving(false)
    }
  }

  return (
    <div className="flex flex-col gap-3">
      <Alert variant="info">
        <AlertIcon>
          <InfoCircleFilled />
        </AlertIcon>
        <AlertTitle>还没有设备身份</AlertTitle>
        <AlertDescription>为这台设备设置一个名称，之后即可参与点歌。</AlertDescription>
      </Alert>
      <form onSubmit={submit} className="flex flex-col gap-3">
        <Field name="display_name">
          <FieldLabel>设备名称</FieldLabel>
          <Input
            value={name}
            onChange={(e) => setName(e.target.value)}
            maxLength={32}
            placeholder="例如：客厅音箱"
            autoComplete="off"
          />
          <FieldDescription>最多 32 个字符，可随时修改。</FieldDescription>
        </Field>
        <div>
          <Button type="submit" disabled={!name.trim() || saving}>
            {saving ? <Spinner currentColor /> : <Check data-icon="start" />}
            {saving ? '保存中…' : '保存'}
          </Button>
        </div>
      </form>
    </div>
  )
}

/** 已登录设备：展示身份，管理员可进入后台或退出登录（需确认）。 */
function DeviceIdentity() {
  const auth = useStore((s) => s.auth)
  const navigate = useNavigate()
  const [confirmOpen, setConfirmOpen] = useState(false)
  const [loggingOut, setLoggingOut] = useState(false)
  const isAdmin = auth?.role === 'admin'

  const handleLogout = async () => {
    if (loggingOut) return
    setLoggingOut(true)
    try {
      await adminLogout()
      useStore.getState().setAuth(null)
      setConfirmOpen(false)
      toast('已退出登录', 'success')
      navigate('/')
    } catch (err) {
      toast(errMsg(err), 'error')
    } finally {
      setLoggingOut(false)
    }
  }

  return (
    <div className="flex flex-col gap-4">
      <div className="flex min-w-0 items-center gap-3">
        <span className="bg-background-muted flex size-10 shrink-0 items-center justify-center rounded-full">
          <UserFilled className="text-foreground-muted size-5" />
        </span>
        <div className="min-w-0">
          <p className="text-foreground-intense truncate text-sm font-semibold">{auth?.display_name ?? '未命名设备'}</p>
          <Badge variant={isAdmin ? 'primary' : 'soft'} size="xs" className="mt-0.5">
            {isAdmin ? <ShieldCheckFilled /> : <UserFilled />}
            {isAdmin ? '管理员' : '普通用户'}
          </Badge>
        </div>
      </div>

      {isAdmin && (
        <div className="flex flex-wrap gap-2">
          <Button variant="outline" render={<Link to="/admin" />}>
            <ExternalLink data-icon="start" />
            管理后台
          </Button>
          <Button variant="destructive" onClick={() => setConfirmOpen(true)}>
            <Logout data-icon="start" />
            退出登录
          </Button>
        </div>
      )}

      <AlertDialog open={confirmOpen} onOpenChange={setConfirmOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>退出登录？</AlertDialogTitle>
            <AlertDialogDescription>
              将清除本设备的登录状态，设备名称需要重新设置后才能继续使用。
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogClose render={<Button variant="outline">取消</Button>} />
            <Button variant="destructive" onClick={handleLogout} disabled={loggingOut}>
              {loggingOut && <Spinner currentColor />}
              {loggingOut ? '退出中…' : '确认退出'}
            </Button>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  )
}

export function DeviceSection() {
  const auth = useStore((s) => s.auth)
  const station = useStore((s) => s.station)
  const needsSetup = station?.needs_setup === true

  return (
    <section aria-labelledby="settings-device-heading" className="rounded-2xl border border-border-muted bg-background-subtle p-4 sm:p-5">
      <h2 id="settings-device-heading" className="mb-4 flex items-center gap-2 text-base font-semibold">
        <DeviceMobile data-icon="start" />
        设备
      </h2>
      <div className="flex flex-col gap-4">
        {needsSetup && <ClaimAdminBlock />}
        {auth ? <DeviceIdentity /> : <DeviceNameForm />}
      </div>
    </section>
  )
}
