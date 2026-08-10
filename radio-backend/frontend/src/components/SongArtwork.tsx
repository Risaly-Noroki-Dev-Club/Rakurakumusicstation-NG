import { useState } from 'react'
import { Thumbnail } from '@appica/ui-react/thumbnail'
import { Music } from '@appica/icons-react'

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
 * player page): request the image only when hasCover is true, fall back to the
 * music glyph on load error. No artificial timeout — slow-but-successful
 * covers must still render.
 */
export function SongArtwork({ hasCover, coverSrc, alt = '', size = 'md', className }: SongArtworkProps) {
  const [failed, setFailed] = useState(false)
  const showImage = hasCover && !failed
  return (
    <Thumbnail
      variant={showImage ? 'image' : 'icon-soft'}
      shape="rounded"
      size={size}
      src={showImage ? coverSrc : undefined}
      alt={alt}
      onLoadingStatusChange={(status) => {
        if (status === 'error') setFailed(true)
      }}
      className={className}
    >
      <Music />
    </Thumbnail>
  )
}
