import { useEffect, useState } from 'react'
import { Thumbnail } from '@appica/ui-react/thumbnail'
import { Music } from '@appica/icons-react'
import { isCoverKnownGood, rememberCoverSuccess } from '@/lib/coverCache'

export interface SongArtworkProps {
  /** Whether the track has artwork (song.has_cover / playback.coverUrl != null). */
  hasCover: boolean
  /** Absolute cover URL (coverUrl(song.id)); only requested when hasCover. */
  coverSrc?: string
  alt?: string
  size?: '2xs' | 'xs' | 'sm' | 'md' | 'lg' | 'xl' | '2xl' | number
  className?: string
}

/**
 * Single cover-artwork convention for every surface (song table, mini player,
 * player page). We preload the image ourselves so the fallback state is fully
 * controlled:
 * - no cover → music glyph, no request at all
 * - cover loading / failed → music glyph (Thumbnail's built-in image-variant
 *   fallback is a broken-image icon and its `children` are ignored)
 * - cover loaded → `variant="image"` (served from the browser cache, no flash)
 */
export function SongArtwork({ hasCover, coverSrc, alt = '', size = 'md', className }: SongArtworkProps) {
  // Covers that loaded successfully before skip the preload round-trip.
  const [imgReady, setImgReady] = useState(() => (hasCover && coverSrc ? isCoverKnownGood(coverSrc) : false))
  const [failed, setFailed] = useState(false)

  useEffect(() => {
    if (!hasCover || !coverSrc) {
      setImgReady(false)
      setFailed(false)
      return
    }
    setImgReady(isCoverKnownGood(coverSrc))
    setFailed(false)
    let cancelled = false
    const im = new Image()
    im.onload = () => {
      if (!cancelled) {
        rememberCoverSuccess(coverSrc)
        setImgReady(true)
      }
    }
    im.onerror = () => {
      if (!cancelled) setFailed(true)
    }
    im.src = coverSrc
    return () => {
      cancelled = true
      im.onload = null
      im.onerror = null
    }
  }, [hasCover, coverSrc])

  const showImage = imgReady && !failed

  return (
    <Thumbnail
      variant={showImage ? 'image' : 'icon-soft'}
      shape="rounded"
      size={size}
      src={showImage ? coverSrc : undefined}
      alt={alt}
      className={className}
    >
      {showImage ? null : <Music />}
    </Thumbnail>
  )
}
