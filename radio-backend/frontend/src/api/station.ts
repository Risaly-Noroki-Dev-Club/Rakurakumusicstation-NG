import { apiBase } from './client'
import { store, applyStationColors, toast } from '../store'

export async function loadStationInfo(): Promise<void> {
  try {
    const res = await fetch(apiBase + '/api/station')
    const data = await res.json()
    if (data) {
      const info = data.data || data
      store.stationName = info.name
      store.needsSetup = info.needs_setup === true
      document.title = info.name
      if (info.primary_color) {
        const metaTheme = document.querySelector('meta[name="theme-color"]') as HTMLMetaElement | null
        if (metaTheme) metaTheme.content = info.primary_color
        document.documentElement.style.setProperty('--primary', info.primary_color)
      }
      applyStationColors(info)
      if (info.stream_url) {
        try {
          const u = new URL(info.stream_url, window.location.origin)
          // audioEngineUrl is managed by api/client.ts
          // eslint-disable-next-line @typescript-eslint/no-var-requires
          const client = require('./client')
          client.setStreamUrl(info.stream_url)
          client.setAudioEngineUrl(u.origin)
        } catch { /* ignore */ }
      }
    }
  } catch { /* ignore */ }
}
