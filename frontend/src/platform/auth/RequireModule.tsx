import type { ReactNode } from 'react'
import { Navigate } from 'react-router-dom'
import { useAuth } from './AuthContext'

interface RequireModuleProps {
  /** Must match the module name the backend's
   * `platform::feature_flags::Module::NAME` uses, e.g. "manufacturing". */
  module: string
  children: ReactNode
}

/** Client-side mirror of the backend's `RequireModule<M>` extractor: gates
 * a route on the tenant's known entitlements instead of waiting for a 403
 * from the API. This is a UX shortcut, not the security boundary — the
 * backend still enforces the real check on every request, and
 * PaywallNotice still handles the case where entitlements changed
 * server-side mid-session and a call 403s anyway. */
export function RequireModule({ module, children }: RequireModuleProps) {
  const { hasModule } = useAuth()

  if (!hasModule(module)) {
    return <Navigate to="/paywall" replace state={{ module }} />
  }

  return <>{children}</>
}
