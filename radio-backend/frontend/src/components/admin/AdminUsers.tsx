import { useCallback, useEffect, useState } from 'react'
import { banUser, fetchAdminUsers, setUserRole, unbanUser } from '@/api'
import type { AdminUser } from '@/types'
import { useStore } from '@/store'
import { Badge } from '@appica/ui-react/badge'
import { Button } from '@appica/ui-react/button'
import { DropdownMenu, DropdownMenuTrigger, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator } from '@appica/ui-react/dropdown-menu'
import { ScrollArea } from '@appica/ui-react/scroll-area'
import { Skeleton } from '@appica/ui-react/skeleton'
import { Table, TableHeader, TableBody, TableRow, TableHead, TableCell } from '@appica/ui-react/table'
import { Ban, Check, Crown, DotsVertical, User } from '@appica/icons-react'
import { formatDateTime } from '@/lib/format'

function errMsg(e: unknown): string {
  return e instanceof Error ? e.message : '操作失败'
}

export function AdminUsers() {
  const addToast = useStore((s) => s.addToast)
  const auth = useStore((s) => s.auth)
  const [users, setUsers] = useState<AdminUser[]>([])
  const [loading, setLoading] = useState(true)
  const [busyId, setBusyId] = useState<number | null>(null)

  const loadUsers = useCallback(async () => {
    try {
      const list = await fetchAdminUsers()
      setUsers(Array.isArray(list) ? list : [])
    } catch (e) {
      addToast(errMsg(e), 'error')
    } finally {
      setLoading(false)
    }
  }, [addToast])

  useEffect(() => {
    void loadUsers()
  }, [loadUsers])

  async function runAction(id: number, action: () => Promise<unknown>, successMsg: string) {
    setBusyId(id)
    try {
      await action()
      addToast(successMsg, 'success')
      await loadUsers()
    } catch (e) {
      addToast(errMsg(e), 'error')
    } finally {
      setBusyId(null)
    }
  }

  const changeRole = (user: AdminUser, role: 'admin' | 'user') =>
    runAction(user.id, () => setUserRole(user.id, role), role === 'admin' ? '已设为管理员' : '已设为普通用户')

  const toggleBan = (user: AdminUser) =>
    user.banned_until
      ? runAction(user.id, () => unbanUser(user.id), '已解封')
      : runAction(user.id, () => banUser(user.id), '已封禁')

  return (
    <div className="space-y-4">
      {loading ? (
        <div className="space-y-2">
          {Array.from({ length: 5 }).map((_, i) => (
            <Skeleton key={i} className="h-10 w-full rounded-lg" />
          ))}
        </div>
      ) : users.length === 0 ? (
        <p className="text-foreground-muted py-8 text-center text-sm">暂无用户</p>
      ) : (
        <ScrollArea orientation="horizontal" scrollShadow className="w-full">
          <Table size="sm" stripedRows hoverableRows className="w-full min-w-[680px]">
            <TableHeader>
              <TableRow>
                <TableHead>用户</TableHead>
                <TableHead className="w-28">角色</TableHead>
                <TableHead className="w-32">状态</TableHead>
                <TableHead className="w-36">注册时间</TableHead>
                <TableHead className="w-16 text-right">操作</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {users.map((user) => {
                const isSelf = auth?.id === user.id
                const banned = Boolean(user.banned_until)
                return (
                  <TableRow key={user.id}>
                    <TableCell className="max-w-[240px] truncate font-medium" title={user.display_name}>
                      {user.display_name}
                      {isSelf && <span className="text-foreground-muted ms-1 text-xs">（我）</span>}
                    </TableCell>
                    <TableCell className="whitespace-nowrap">
                      <Badge variant={user.role === 'admin' ? 'primary' : 'soft'} size="xs">
                        {user.role === 'admin' ? '管理员' : '普通用户'}
                      </Badge>
                    </TableCell>
                    <TableCell className="whitespace-nowrap">
                      {banned ? (
                        <Badge variant="error" size="xs" title={`封禁至 ${formatDateTime(user.banned_until ?? '')}`}>
                          封禁中
                        </Badge>
                      ) : (
                        <Badge variant="success" size="xs">正常</Badge>
                      )}
                    </TableCell>
                    <TableCell className="text-foreground-muted whitespace-nowrap">
                      {formatDateTime(user.created_at)}
                    </TableCell>
                    <TableCell>
                      <div className="flex justify-end">
                        <DropdownMenu>
                          <DropdownMenuTrigger
                            render={
                              <Button variant="ghost" size="icon-sm" aria-label={`操作 ${user.display_name}`} disabled={isSelf}>
                                <DotsVertical />
                              </Button>
                            }
                          />
                          <DropdownMenuContent align="end">
                            <DropdownMenuItem disabled={isSelf} onClick={() => changeRole(user, 'admin')}>
                              <Crown data-icon="start" />
                              设为管理员
                            </DropdownMenuItem>
                            <DropdownMenuItem disabled={isSelf} onClick={() => changeRole(user, 'user')}>
                              <User data-icon="start" />
                              设为普通用户
                            </DropdownMenuItem>
                            <DropdownMenuSeparator />
                            {banned ? (
                              <DropdownMenuItem disabled={isSelf} onClick={() => toggleBan(user)}>
                                <Check data-icon="start" />
                                解封
                              </DropdownMenuItem>
                            ) : (
                              <DropdownMenuItem
                                disabled={isSelf}
                                className="text-error-emphasis! data-highlighted:before:bg-error-subtle!"
                                onClick={() => toggleBan(user)}
                              >
                                <Ban data-icon="start" />
                                封禁
                              </DropdownMenuItem>
                            )}
                          </DropdownMenuContent>
                        </DropdownMenu>
                        {busyId === user.id && (
                          <span className="text-foreground-muted flex items-center text-xs">处理中…</span>
                        )}
                      </div>
                    </TableCell>
                  </TableRow>
                )
              })}
            </TableBody>
          </Table>
        </ScrollArea>
      )}
    </div>
  )
}
