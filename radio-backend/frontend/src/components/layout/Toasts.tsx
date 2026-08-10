import { createPortal } from 'react-dom'
import { Alert, AlertIcon, AlertTitle } from '@appica/ui-react/alert'
import { InfoCircleFilled, CircleCheckFilled, AlertTriangleFilled, CircleXFilled } from '@appica/icons-react'
import { useStore, type Toast } from '@/store'

const ICONS = {
  info: InfoCircleFilled,
  success: CircleCheckFilled,
  warning: AlertTriangleFilled,
  error: CircleXFilled,
} as const

const VARIANTS = {
  info: 'info',
  success: 'success',
  warning: 'warning',
  error: 'error',
} as const

function ToastItem({ toast }: { toast: Toast }) {
  const removeToast = useStore((s) => s.removeToast)
  const Icon = ICONS[toast.level]
  return (
    <Alert
      variant={VARIANTS[toast.level]}
      open
      onOpenChange={(open) => {
        if (!open) removeToast(toast.id)
      }}
      dismissible
      className="pointer-events-auto w-full"
    >
      <AlertIcon>
        <Icon />
      </AlertIcon>
      <AlertTitle className="text-sm font-medium">{toast.message}</AlertTitle>
    </Alert>
  )
}

/** Fixed toast stack (bottom-right, above the mini player). */
export function Toasts() {
  const toasts = useStore((s) => s.toasts)
  return createPortal(
    <div className="pointer-events-none fixed right-4 bottom-20 z-50 flex w-80 max-w-[calc(100vw-2rem)] flex-col gap-2">
      {toasts.map((toast) => (
        <ToastItem key={toast.id} toast={toast} />
      ))}
    </div>,
    document.body,
  )
}
