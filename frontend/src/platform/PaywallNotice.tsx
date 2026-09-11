import { useEffect, useState } from 'react'
import { PAYWALL_EVENT, type PaywallEventDetail } from '../api/client'
import { Toast } from '../components/ui/Toast'

/** Mount once near the root. Listens for the `PAYWALL_EVENT` that
 * api/client.ts dispatches on every 403 response and surfaces it as a
 * transient toast — the mid-session counterpart to PaywallPage, for when
 * an already-rendered view calls an endpoint the tenant isn't entitled to
 * (e.g. entitlements changed server-side since the page loaded). */
export function PaywallNotice() {
  const [notice, setNotice] = useState<PaywallEventDetail | null>(null)

  useEffect(() => {
    const onPaywall = (event: Event) => {
      setNotice((event as CustomEvent<PaywallEventDetail>).detail)
    }

    window.addEventListener(PAYWALL_EVENT, onPaywall)
    return () => window.removeEventListener(PAYWALL_EVENT, onPaywall)
  }, [])

  if (!notice) return null

  return (
    <Toast tone="warning" onDismiss={() => setNotice(null)}>
      {notice.message}
    </Toast>
  )
}
