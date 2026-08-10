import { useNavigate } from 'react-router-dom'
import { Button } from '@appica/ui-react/button'
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@appica/ui-react/tabs'
import { LayoutDashboard, Lock } from '@appica/icons-react'
import { useStore } from '@/store'
import { AdminOverview } from '@/components/admin/AdminOverview'
import { AdminSongs } from '@/components/admin/AdminSongs'
import { AdminUsers } from '@/components/admin/AdminUsers'
import { AdminDownloads } from '@/components/admin/AdminDownloads'
import { AdminNcm } from '@/components/admin/AdminNcm'
import { AdminStationSettings } from '@/components/admin/AdminStationSettings'

export default function AdminPage() {
  const auth = useStore((s) => s.auth)
  const navigate = useNavigate()

  if (auth?.role !== 'admin') {
    return (
      <div className="flex min-h-full flex-col items-center justify-center gap-4 px-4 py-20 text-center">
        <div className="bg-error-subtle text-error-emphasis flex size-16 items-center justify-center rounded-full">
          <Lock className="size-7" aria-hidden />
        </div>
        <h1 className="text-xl font-semibold">仅管理员可访问</h1>
        <Button variant="secondary" onClick={() => navigate('/')}>
          返回首页
        </Button>
      </div>
    )
  }

  return (
    <div className="mx-auto w-full max-w-6xl px-4 py-6">
      <h1 className="mb-4 flex items-center gap-2 text-2xl font-semibold">
        <LayoutDashboard className="text-primary size-5.5" aria-hidden="true" />
        管理后台
      </h1>
      <Tabs variant="line" defaultValue="overview">
        <TabsList className="w-full overflow-x-auto">
          <TabsTrigger value="overview">概览</TabsTrigger>
          <TabsTrigger value="songs">歌曲</TabsTrigger>
          <TabsTrigger value="users">用户</TabsTrigger>
          <TabsTrigger value="downloads">下载</TabsTrigger>
          <TabsTrigger value="ncm">网易云</TabsTrigger>
          <TabsTrigger value="settings">电台设置</TabsTrigger>
        </TabsList>
        <TabsContent value="overview" className="pt-4">
          <AdminOverview />
        </TabsContent>
        <TabsContent value="songs" className="pt-4">
          <AdminSongs />
        </TabsContent>
        <TabsContent value="users" className="pt-4">
          <AdminUsers />
        </TabsContent>
        <TabsContent value="downloads" className="pt-4">
          <AdminDownloads />
        </TabsContent>
        <TabsContent value="ncm" className="pt-4">
          <AdminNcm />
        </TabsContent>
        <TabsContent value="settings" className="pt-4">
          <AdminStationSettings />
        </TabsContent>
      </Tabs>
    </div>
  )
}
