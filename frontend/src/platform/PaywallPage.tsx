import { useLocation, Link } from 'react-router-dom'
import { Card } from '../components/ui/Card'
import { Button } from '../components/ui/Button'

interface LocationState {
  module?: string
}

/** Landing spot for RequireModule's redirect — a full-page notice for
 * "you navigated to a route this plan doesn't include," as opposed to
 * PaywallNotice, which is a transient toast for a 403 that happens
 * mid-session on an already-rendered page. */
export function PaywallPage() {
  const location = useLocation()
  const module = (location.state as LocationState | null)?.module

  return (
    <div className="flex min-h-screen items-center justify-center bg-slate-950 px-4">
      <Card className="w-full max-w-md text-center">
        <h1 className="mb-2 text-xl font-semibold text-slate-100">Upgrade required</h1>
        <p className="mb-6 text-slate-400">
          {module
            ? `The "${module}" module isn't included in your current plan.`
            : "This feature isn't included in your current plan."}
        </p>
        <Link to="/">
          <Button className="w-full">Back to dashboard</Button>
        </Link>
      </Card>
    </div>
  )
}
