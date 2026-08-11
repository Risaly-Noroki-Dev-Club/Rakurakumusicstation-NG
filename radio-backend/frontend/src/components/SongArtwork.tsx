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
  shape?: 'circle' | 'rounded'
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
/**
 * 所有曲目图标统一方形（用户要求：不区分大小一律方形）。
 * Appica Thumbnail 的 shape="rounded" 是 squircle（≈35% 圆角），
 * 小图上接近圆形；这里按图尺寸分级给固定像素圆角，
 * 用 inline style 确保覆盖 squircle（不依赖 tailwind-merge 对复杂 calc 类的识别）。
 */
function cornerPxFor(size: SongArtworkProps['size']): number {
  const px =
    typeof size === 'number'
      ? size
      : { '2xs': 20, xs: 24, sm: 32, md: 40, lg: 48, xl: 64, '2xl': 80 }[size as string] ?? 40
  if (px <= 48) return 6
  if (px <= 96) return 8
  return 16
}

export function SongArtwork({ hasCover, coverSrc, alt = '', size = 'md', shape = 'rounded', className }: SongArtworkProps) {
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
      shape={shape}
      size={size}
      src={showImage ? coverSrc : undefined}
      alt={alt}
      style={shape === 'rounded' ? { borderRadius: cornerPxFor(size) } : undefined}
      className={className}
    >
      {showImage ? null : <Music />}
    </Thumbnail>
  )
}
