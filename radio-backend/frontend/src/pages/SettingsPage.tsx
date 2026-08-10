import { DeviceSection } from '@/components/settings/DeviceSection'
import { AppearanceSection } from '@/components/settings/AppearanceSection'
import { NcmSection } from '@/components/settings/NcmSection'

export default function SettingsPage() {
  return (
    <div className="mx-auto w-full max-w-3xl px-4 py-6 sm:px-6">
      <header className="mb-5">
        <h1 className="text-foreground-intense text-2xl font-semibold tracking-tight">设置</h1>
        <p className="text-foreground-muted mt-1 text-sm">管理设备身份、外观主题与账号信息</p>
      </header>
      <div className="flex flex-col gap-4">
        <DeviceSection />
        <AppearanceSection />
        <NcmSection />
      </div>
    </div>
  )
}
