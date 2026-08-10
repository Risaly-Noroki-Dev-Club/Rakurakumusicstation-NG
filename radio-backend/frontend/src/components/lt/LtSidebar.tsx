import { Avatar, AvatarFallback, AvatarGroup } from '@appica/ui-react/avatar'
import { Badge } from '@appica/ui-react/badge'
import { Separator } from '@appica/ui-react/separator'
import { Users } from '@appica/icons-react'
import { useStore } from '@/store'

/** 正在收听 — presence sidebar (listener names from WS listeners_update). */
export function LtSidebar() {
  const names = useStore((s) => s.listeners.names)
  const count = useStore((s) => s.listeners.count)
  const displayName = useStore((s) => s.auth?.display_name ?? '匿名')

  return (
    <aside className="border-border-muted flex shrink-0 flex-col gap-4 border-b p-4 md:w-64 md:border-b-0 md:border-r">
      <div className="flex items-center gap-2">
        <Users data-icon="start" className="text-primary" />
        <h2 className="text-foreground-intense font-semibold">正在收听</h2>
        <Badge variant="secondary" aria-label={`共 ${count} 人在线`}>
          {count}
        </Badge>
      </div>

      {names.length > 0 ? (
        <AvatarGroup size="md" className="flex-wrap">
          {names.map((name) => (
            <Avatar key={name} title={name}>
              <AvatarFallback>{name.trim().charAt(0).toUpperCase() || '?'}</AvatarFallback>
            </Avatar>
          ))}
        </AvatarGroup>
      ) : (
        <p className="text-foreground-muted text-sm">还没有其他听众，快邀请朋友一起来吧</p>
      )}

      <Separator />

      <div className="flex items-center gap-2">
        <Avatar size="sm">
          <AvatarFallback className="bg-primary text-primary-foreground">
            {displayName.trim().charAt(0).toUpperCase() || '我'}
          </AvatarFallback>
        </Avatar>
        <div className="min-w-0">
          <p className="text-foreground-intense truncate text-sm font-medium">{displayName}</p>
          <p className="text-foreground-muted text-xs">我 · 正在收听</p>
        </div>
      </div>
    </aside>
  )
}
