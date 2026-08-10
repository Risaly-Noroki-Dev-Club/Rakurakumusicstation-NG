import type { AriaAttributes } from 'react'

/**
 * Native determinate progress bar.
 *
 * Replaces @appica/ui-react/progress, which wraps Base UI Progress.Root —
 * the same version-drift crash class as Field ("Base UI error #29"-style
 * ProgressRootContext missing). A plain track + indicator div cannot break.
 */
export type ProgressProps = {
  value: number
  className?: string
  'aria-label'?: AriaAttributes['aria-label']
}

export function Progress({ value, className, ...rest }: ProgressProps) {
  const pct = Math.max(0, Math.min(100, value))
  return (
    <div
      role="progressbar"
      aria-valuenow={Math.round(pct)}
      aria-valuemin={0}
      aria-valuemax={100}
      className={`bg-background-strong relative h-1.5 w-full overflow-hidden rounded-full ${className ?? ''}`}
      {...rest}
    >
      <div
        className="bg-primary h-full rounded-full transition-[width] duration-300 motion-reduce:transition-none"
        style={{ width: `${pct}%` }}
      />
    </div>
  )
}
