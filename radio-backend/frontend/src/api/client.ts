import type { ApiResponse } from '@/types'

/** URL root: origin + BASE_URL (e.g. https://host/radio) with no trailing slash. */
export function appRoot(): string {
  return (window.location.origin + import.meta.env.BASE_URL).replace(/\/+$/, '')
}

/** Absolute URL for a server path starting with "/". */
export function appUrl(path: string): string {
  return appRoot() + path
}

/** WebSocket URL for a server path starting with "/". */
export function wsUrl(path: string): string {
  const proto = window.location.protocol === 'https:' ? 'wss' : 'ws'
  return `${proto}://${window.location.host}${import.meta.env.BASE_URL.replace(/\/+$/, '')}${path}`
}

export class ApiError extends Error {
  constructor(
    message: string,
    public status: number,
  ) {
    super(message)
    this.name = 'ApiError'
  }
}

/** Fetch a JSON endpoint, unwrap {success, data, error}, throw ApiError on failure. */
export async function apiFetch<T>(path: string, init?: RequestInit): Promise<T> {
  let res: Response
  try {
    res = await fetch(appUrl(path), {
      credentials: 'same-origin',
      ...init,
      headers: init?.body instanceof FormData ? init?.headers : { 'Content-Type': 'application/json', ...init?.headers },
    })
  } catch {
    throw new ApiError('网络错误，无法连接服务器', 0)
  }
  if (res.status === 401) {
    throw new ApiError('需要登录', 401)
  }
  let body: unknown
  try {
    body = await res.json()
  } catch {
    throw new ApiError(`服务器返回了无法解析的响应 (${res.status})`, res.status)
  }
  const wrapped = body as ApiResponse<T>
  if (wrapped && typeof wrapped.success === 'boolean') {
    if (!wrapped.success) {
      throw new ApiError(wrapped.error ?? '请求失败', res.status)
    }
    return wrapped.data as T
  }
  // Some endpoints (station, now-playing, listeners) return bare JSON.
  return body as T
}

/** Fetch raw bytes (blob) for covers/downloads. */
export async function apiBlob(path: string, init?: RequestInit): Promise<Blob> {
  const res = await fetch(appUrl(path), { credentials: 'same-origin', ...init })
  if (!res.ok) throw new ApiError(`请求失败 (${res.status})`, res.status)
  return res.blob()
}

/** Consume an SSE stream from the backend, invoking onEvent per data line. */
export async function consumeSse(
  path: string,
  onEvent: (data: unknown) => void,
  signal?: AbortSignal,
): Promise<void> {
  const res = await fetch(appUrl(path), { credentials: 'same-origin', signal })
  if (!res.ok || !res.body) throw new ApiError(`SSE 连接失败 (${res.status})`, res.status)
  const reader = res.body.getReader()
  const decoder = new TextDecoder()
  let buffer = ''
  for (;;) {
    const { done, value } = await reader.read()
    if (done) break
    buffer += decoder.decode(value, { stream: true })
    const lines = buffer.split('\n')
    buffer = lines.pop() ?? ''
    for (const line of lines) {
      const trimmed = line.trim()
      if (!trimmed.startsWith('data:')) continue
      try {
        onEvent(JSON.parse(trimmed.slice(5).trim()))
      } catch {
        // ignore malformed SSE frames
      }
    }
  }
}
