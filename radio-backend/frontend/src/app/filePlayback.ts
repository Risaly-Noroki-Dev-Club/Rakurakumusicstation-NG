import { apiFetch, getStreamUrl } from '../api/client'
import { store, toast } from '../store'

let mediaSource: MediaSource | null = null
let sourceBuffer: SourceBuffer | null = null
let fileOffset = 0
let fileFetchActive = false

export function switchPlaybackMode(audioEl: HTMLAudioElement): void {
  store.useFileMode = !store.useFileMode
  if (store.useFileMode) {
    if (typeof MediaSource !== 'undefined') {
      startFilePlayback(audioEl)
    } else {
      toast('你的浏览器不支持推文件模式，使用推流模式', 'error')
      store.useFileMode = false
    }
  } else {
    cleanupFilePlayback()
    audioEl.src = getStreamUrl()
    audioEl.load()
    audioEl.play().catch(() => {})
  }
}

export function cleanupFilePlayback(): void {
  fileFetchActive = false
  if (sourceBuffer) {
    try { mediaSource?.removeSourceBuffer(sourceBuffer) } catch {}
    sourceBuffer = null
  }
  if (mediaSource) {
    try { if (mediaSource.readyState === 'open') mediaSource.endOfStream() } catch {}
    mediaSource = null
  }
}

function startFilePlayback(audio: HTMLAudioElement): void {
  cleanupFilePlayback()
  fileOffset = 0
  fileFetchActive = true
  mediaSource = new MediaSource()
  audio.src = URL.createObjectURL(mediaSource)
  mediaSource.addEventListener('sourceopen', async () => {
    try {
      sourceBuffer = mediaSource!.addSourceBuffer('audio/mpeg')
      sourceBuffer.addEventListener('updateend', onSourceBufferUpdateEnd)
      sourceBuffer.addEventListener('error', () => {
        toast('推文件模式解码错误，回退到推流模式', 'error')
        store.useFileMode = false
        cleanupFilePlayback()
        audio.src = getStreamUrl()
        audio.load()
        audio.play().catch(() => {})
      })
      fetchNextFileChunk()
    } catch {
      toast('推文件模式初始化失败，回退到推流模式', 'error')
      store.useFileMode = false
      cleanupFilePlayback()
      audio.src = getStreamUrl()
      audio.load()
      audio.play().catch(() => {})
    }
  })
}

function onSourceBufferUpdateEnd(): void {
  if (fileFetchActive && sourceBuffer && !sourceBuffer.updating) {
    fetchNextFileChunk()
  }
}

async function fetchNextFileChunk(): Promise<void> {
  if (!fileFetchActive || store.playbackState.song_id <= 0) return
  try {
    const res = await apiFetch('/api/songs/' + store.playbackState.song_id + '/file', {
      headers: { 'Range': 'bytes=' + fileOffset + '-' }
    })
    if (!res.ok) {
      endFileStream()
      return
    }
    const buffer = await res.arrayBuffer()
    if (buffer.byteLength === 0) {
      endFileStream()
      return
    }
    fileOffset += buffer.byteLength
    if (sourceBuffer && !sourceBuffer.updating) {
      sourceBuffer.appendBuffer(buffer)
    }
  } catch {
    endFileStream()
  }
}

function endFileStream(): void {
  fileFetchActive = false
  if (mediaSource && mediaSource.readyState === 'open') {
    try { mediaSource.endOfStream() } catch {}
  }
}
