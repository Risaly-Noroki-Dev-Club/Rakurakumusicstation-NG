import type { ComponentProps } from 'react'

/**
 * Native <textarea> styled like Appica's input variants.
 *
 * Replaces @appica/ui-react/textarea, which renders Base UI Field.Control
 * and therefore requires a Field.Root ancestor (Base UI error #28 when
 * missing / when Base UI versions drift).
 */
export type TextareaProps = ComponentProps<'textarea'> & {
  variant?: 'outline' | 'soft'
  inputSize?: 'sm' | 'md' | 'lg'
}

const base =
  'w-full border text-foreground placeholder:text-foreground-subtle outline-none resize-y transition-[background-color,border-color,box-shadow] duration-200 motion-reduce:transition-none'

const variants: Record<NonNullable<TextareaProps['variant']>, string> = {
  outline: 'bg-background border-border-strong',
  soft: 'bg-background-muted border-transparent',
}

const sizes: Record<NonNullable<TextareaProps['inputSize']>, string> = {
  sm: 'min-h-16 px-3 py-2 text-xs rounded-sm',
  md: 'min-h-20 px-3.5 py-2.5 text-sm rounded-md',
  lg: 'min-h-24 px-4 py-3 text-base rounded-lg',
}

export function Textarea({
  className,
  variant = 'outline',
  inputSize = 'md',
  ...props
}: TextareaProps) {
  return (
    <textarea
      className={`${base} ${variants[variant]} ${sizes[inputSize]} focus-visible:ring-3 focus-visible:ring-ring-input focus-visible:border-transparent focus-visible:bg-background disabled:border-border-strong! disabled:bg-background-subtle! disabled:cursor-not-allowed disabled:border-dashed disabled:opacity-disabled ${className ?? ''}`}
      {...props}
    />
  )
}
