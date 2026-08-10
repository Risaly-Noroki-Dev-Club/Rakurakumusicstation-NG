import { NavLink, useLocation } from 'react-router-dom'
import { Navigation, NavigationList, NavigationItem, NavigationLink } from '@appica/ui-react/navigation'
import { Radio, Music, Playlist, History, Library, Users, Settings } from '@appica/icons-react'
import { useStore } from '@/store'
import { cn } from '@/lib/cn'

export function MainNav() {
  const location = useLocation()
  const isAdmin = useStore((s) => s.auth?.role === 'admin')
  const isListen = location.pathname.startsWith('/listen')

  const items = [
    { to: '/', label: '正在播放', icon: Radio },
    { to: '/player', label: '播放器', icon: Music },
    { to: '/queue', label: '点歌队列', icon: Playlist },
    { to: '/up-next', label: '接下来', icon: History },
    { to: '/library', label: '曲库', icon: Library },
    { to: '/listen', label: '一起听', icon: Users },
    { to: '/settings', label: '设置', icon: Settings },
  ]

  if (isAdmin) items.push({ to: '/admin', label: '管理', icon: Settings })

  const activeFor = (to: string) => (to === '/' ? location.pathname === '/' : location.pathname.startsWith(to))

  return (
    <div className={cn('min-w-0', isListen && 'lt-nav')}>
      <Navigation orientation="horizontal" variant="pill" size="sm" aria-label="主导航" className="max-w-full overflow-x-auto">
        <NavigationList className="flex items-center gap-1">
          {items.map(({ to, label, icon: Icon }) => (
            <NavigationItem key={to}>
              <NavigationLink
                render={<NavLink to={to} />}
                active={activeFor(to)}
                className="whitespace-nowrap"
              >
                <Icon data-icon="start" />
                {label}
              </NavigationLink>
            </NavigationItem>
          ))}
        </NavigationList>
      </Navigation>
    </div>
  )
}
