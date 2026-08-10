import { NavLink, useLocation } from 'react-router-dom'
import { Navigation, NavigationList, NavigationItem, NavigationLink } from '@appica/ui-react/navigation'
import { Music, Library, Settings, LayoutDashboard } from '@appica/icons-react'
import { useStore } from '@/store'

export function MainNav() {
  const location = useLocation()
  const isAdmin = useStore((s) => s.auth?.role === 'admin')

  const items = [
    { to: '/player', label: '播放器', icon: Music },
    { to: '/library', label: '曲库', icon: Library },
    { to: '/settings', label: '设置', icon: Settings },
  ]

  if (isAdmin) items.push({ to: '/admin', label: '管理', icon: LayoutDashboard })

  const activeFor = (to: string) => location.pathname === to || location.pathname.startsWith(`${to}/`)

  return (
    <div className="min-w-0">
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
