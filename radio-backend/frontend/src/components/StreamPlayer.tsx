import { useEffect } from 'react'
import { Button } from '@appica/ui-react/button'
import { PlayerPlay } from '@appica/icons-react'
import { useStore } from '@/store'
import { resumeAudio, syncAudio } from '@/audio/streamAudio'

/**
 * Headless component that keeps the singleton <audio> element in sync with
 * the store and renders the autoplay-unblock overlay when the browser
 * blocked playback (needs a user gesture). Once the user clicks, the audio
 * controller marks the session authorized and never shows the overlay again.
 */
export function StreamPlayer() {
  const needsPlay = useStore((s) => s.needsPlay)
  const status = useStore((s) => s.playback?.status)

  useEffect(() => {
    syncAudio()
    return useStore.subscribe(syncAudio)
  }, [])

  if (!needsPlay || status !== 'playing') return null

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-background/70 backdrop-blur-sm">
      <div className="flex flex-col items-center gap-4">
        <p className="text-foreground-muted text-sm">浏览器阻止了自动播放</p>
        <Button size="lg" onClick={resumeAudio}>
          <PlayerPlay data-icon="start" />
          开始收听
        </Button>
      </div>
    </div>
  )
}
