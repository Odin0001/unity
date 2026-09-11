import type { ReactNode } from 'react'

type Tone = 'info' | 'warning' | 'error'

interface ToastProps {
  tone?: Tone
  children: ReactNode
  onDismiss?: () => void
}

const toneClasses: Record<Tone, string> = {
  info: 'border-slate-700 bg-slate-800 text-slate-100',
  warning: 'border-amber-700 bg-amber-950 text-amber-100',
  error: 'border-red-800 bg-red-950 text-red-100',
}

export function Toast({ tone = 'info', children, onDismiss }: ToastProps) {
  return (
    <div
      role="alert"
      className={`fixed bottom-4 right-4 z-50 flex max-w-sm items-start gap-3 rounded-md border px-4 py-3 shadow-xl ${toneClasses[tone]}`}
    >
      <p className="text-sm">{children}</p>
      {onDismiss && (
        <button
          type="button"
          onClick={onDismiss}
          aria-label="Dismiss"
          className="ml-auto text-sm text-current opacity-70 hover:opacity-100"
        >
          ×
        </button>
      )}
    </div>
  )
}
