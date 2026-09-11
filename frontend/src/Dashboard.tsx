import { Card } from './components/ui/Card'
import { Button } from './components/ui/Button'
import { useAuth } from './platform/auth/AuthContext'

/** Temporary home route. `modules/inventory` and `modules/manufacturing`
 * are intentionally empty scaffolding (mirroring the backend) — this
 * placeholder stands in until those modules add their own routed views. */
export function Dashboard() {
  const { session, logout } = useAuth()

  return (
    <div className="min-h-screen bg-slate-950 px-6 py-10 text-slate-100">
      <div className="mx-auto flex max-w-2xl flex-col gap-6">
        <div className="flex items-center justify-between">
          <h1 className="text-2xl font-semibold">Tenants ERP</h1>
          <Button variant="secondary" onClick={logout}>
            Sign out
          </Button>
        </div>

        <Card>
          <h2 className="mb-2 text-sm font-medium uppercase tracking-wide text-slate-400">
            Session
          </h2>
          <dl className="space-y-1 text-sm">
            <div className="flex justify-between">
              <dt className="text-slate-400">Tenant</dt>
              <dd className="font-mono text-slate-200">{session?.tenantId}</dd>
            </div>
            <div className="flex justify-between">
              <dt className="text-slate-400">Entitlements</dt>
              <dd className="font-mono text-slate-200">
                {session?.entitlements.length ? session.entitlements.join(', ') : 'none'}
              </dd>
            </div>
          </dl>
        </Card>

        <p className="text-sm text-slate-500">
          <code className="text-slate-400">src/modules/inventory</code> and{' '}
          <code className="text-slate-400">src/modules/manufacturing</code> are empty — add
          feature routes there as those modules land on the backend.
        </p>
      </div>
    </div>
  )
}
