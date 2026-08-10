/** Format milliseconds as m:ss or h:mm:ss. */
export function formatTime(ms: number): string {
  if (!Number.isFinite(ms) || ms < 0) return '0:00'
  const total = Math.floor(ms / 1000)
  const h = Math.floor(total / 3600)
  const m = Math.floor((total % 3600) / 60)
  const s = total % 60
  const ss = String(s).padStart(2, '0')
  return h > 0 ? `${h}:${String(m).padStart(2, '0')}:${ss}` : `${m}:${ss}`
}

/** Format bytes as a human-readable size. */
export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB']
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1)
  const v = bytes / 1024 ** i
  return `${v >= 100 || i === 0 ? Math.round(v) : v.toFixed(1)} ${units[i]}`
}

/**
 * Format an RFC3339 timestamp as a short locale string.
 *
 * 后端所有时间来自 SQLite datetime('now')，是 UTC 且无时区标记
 * （NaiveDateTime → "2026-08-10 23:24:00"）。无时区串会被浏览器按本地
 * 时区解析，导致显示差 8 小时——统一按 UTC 补 Z 后再转本地时区。
 */
export function formatDateTime(iso: string): string {
  const raw = /^\d{4}-\d{2}-\d{2}[ T]\d{2}:\d{2}:\d{2}$/.test(iso) ? `${iso.replace(' ', 'T')}Z` : iso
  const d = new Date(raw)
  if (Number.isNaN(d.getTime())) return iso
  return d.toLocaleString(undefined, {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  })
}
