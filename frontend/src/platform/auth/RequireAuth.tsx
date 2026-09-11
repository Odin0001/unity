import { Navigate, Outlet, useLocation } from 'react-router-dom'
import { Spinner } from '../../components/ui/Spinner'
import { useAuth } from './AuthContext'

/** Route guard for `<Route element={<RequireAuth />}>` parent routes.
 * Renders nothing but a spinner until the initial session check resolves,
 * then either renders the matched child route or redirects to /login,
 * preserving where the user was headed so login can send them back. */
export function RequireAuth() {
  const { isAuthenticated, isLoading } = useAuth()
  const location = useLocation()

  if (isLoading) {
    return (
      <div className="flex min-h-screen items-center justify-center">
        <Spinner />
      </div>
    )
  }

  if (!isAuthenticated) {
    return <Navigate to="/login" replace state={{ from: location }} />
  }

  return <Outlet />
}
