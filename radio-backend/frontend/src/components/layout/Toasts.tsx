import { ToastProvider, Toaster } from '@appica/ui-react/toast'
import { toastManager } from '@/lib/toast'

/** Appica Toast stack (bottom-right, above the mini player), driven by the shared toastManager. */
export function Toasts() {
  return (
    <ToastProvider toastManager={toastManager} timeout={4000}>
      <Toaster position="bottom-right" progress className="bottom-20" />
    </ToastProvider>
  )
}
