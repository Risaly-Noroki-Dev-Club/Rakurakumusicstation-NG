import type { ComponentProps } from 'react'

/**
 * Native form-field wrapper (label + hint paragraph).
 *
 * Replaces @appica/ui-react/field, which is a thin wrapper over
 * @base-ui/react/field — Base UI version drift (1.7) made Field.Root
 * render without its context and crash every Field part ("Base UI error
 * #28"). A plain div/label/p has no such coupling.
 */
export function Field({ className, name: _name, ...props }: ComponentProps<'div'> & { name?: string }) {
  return <div className={className} {...props} />
}

export function FieldLabel({ className, ...props }: ComponentProps<'label'>) {
  return (
    <label
      className={`text-foreground-intense mb-1.5 flex w-fit items-center gap-1.5 text-sm font-medium select-none ${className ?? ''}`}
      {...props}
    />
  )
}

export function FieldDescription({ className, ...props }: ComponentProps<'p'>) {
  return <p className={`text-foreground-muted mt-1.5 text-sm ${className ?? ''}`} {...props} />
}
