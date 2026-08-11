// Appica/Base UI toast manager singleton — usable outside React (e.g. the
// zustand store). Rendered by src/components/layout/Toasts.tsx.

import { createToastManager } from '@appica/ui-react/toast'
import type { ReactNode } from 'react'
import { InfoCircleFilled, CircleCheckFilled, AlertTriangleFilled, CircleXFilled } from '@appica/icons-react'

export type ToastLevel = 'info' | 'success' | 'warning' | 'error'

export interface ToastData {
  level: ToastLevel
  icon: ReactNode
}

const LEVEL_ICONS: Record<ToastLevel, typeof InfoCircleFilled> = {
  info: InfoCircleFilled,
  success: CircleCheckFilled,
  warning: AlertTriangleFilled,
  error: CircleXFilled,
}

const LEVEL_COLORS: Record<ToastLevel, string> = {
  info: 'text-info-emphasis',
  success: 'text-success-emphasis',
  warning: 'text-warning-emphasis',
  error: 'text-error-emphasis',
}

export const toastManager = createToastManager<ToastData>()

/** Fire-and-forget toast; errors get high priority and a longer timeout. */
export function showToast(message: string, level: ToastLevel = 'info') {
  const Icon = LEVEL_ICONS[level]
  toastManager.add({
    title: message,
    data: { level, icon: <Icon className={LEVEL_COLORS[level]} aria-hidden="true" /> },
    timeout: level === 'error' ? 6000 : 4000,
    priority: level === 'error' ? 'high' : 'low',
  })
}
