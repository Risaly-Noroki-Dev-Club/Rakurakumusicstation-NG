import { apiFetch, setStreamUrl, setAudioEngineUrl } from './client'
import { store } from '../store'

function setLink(rel: string, href: string) {
  let link = document.querySelector(`link[rel="${rel}"]`) as HTMLLinkElement | null
  if (!link) {
    link = document.createElement('link')
    link.rel = rel
    document.head.appendChild(link)
  }
  link.href = href
}

export async function loadStationInfo(): Promise<void> {
  try {
    const res = await apiFetch('/api/station')
    const data = await res.json()
    if (data) {
      const info = data.data || data
      store.stationName = info.name
      store.needsSetup = info.needs_setup === true
      document.title = info.name
      if (info.icon_url) {
        setLink('icon', info.icon_url)
        setLink('apple-touch-icon', info.icon_url)
      }
      if (info.manifest_url) {
        setLink('manifest', info.manifest_url)
      }
      if (info.stream_url) {
        try {
          const u = new URL(info.stream_url, window.location.origin)
          setStreamUrl(info.stream_url)
          setAudioEngineUrl(u.origin)
        } catch { /* ignore */ }
      }
    }
  } catch { /* ignore */ }
}
