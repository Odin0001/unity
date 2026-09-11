import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from 'react'
import { apiClient } from '../../api/client'
import { clearAccessToken, getAccessToken, setAccessToken } from './tokenStorage'

export interface TenantSession {
  tenantId: string
  userId: string
  entitlements: string[]
}

interface LoginRequest {
  email: string
  password: string
}

interface LoginResponse {
  token: string
  session: TenantSession
}

interface AuthContextValue {
  session: TenantSession | null
  isAuthenticated: boolean
  /** True until the initial "is there already a valid stored token"
   * check resolves — route guards should not redirect while this is
   * true, or a signed-in user gets bounced to /login on every refresh. */
  isLoading: boolean
  login: (credentials: LoginRequest) => Promise<void>
  logout: () => void
  hasModule: (moduleName: string) => boolean
}

const AuthContext = createContext<AuthContextValue | undefined>(undefined)

export function AuthProvider({ children }: { children: ReactNode }) {
  const [session, setSession] = useState<TenantSession | null>(null)
  const [isLoading, setIsLoading] = useState(true)

  useEffect(() => {
    const token = getAccessToken()
    if (!token) {
      setIsLoading(false)
      return
    }

    // The token is opaque by design (see backend platform::auth::OpaqueAuth)
    // — it can't be decoded client-side to check validity or expiry, so a
    // stored token can only be confirmed by asking the backend for the
    // session it maps to. A 401 here is handled globally by api/client.ts.
    apiClient
      .get<TenantSession>('/session')
      .then(setSession)
      .catch(() => setSession(null))
      .finally(() => setIsLoading(false))
  }, [])

  const login = async (credentials: LoginRequest) => {
    const response = await apiClient.post<LoginResponse>('/auth/login', credentials, { skipAuth: true })
    setAccessToken(response.token)
    setSession(response.session)
  }

  const logout = () => {
    clearAccessToken()
    setSession(null)
  }

  const hasModule = (moduleName: string) => session?.entitlements.includes(moduleName) ?? false

  const value = useMemo<AuthContextValue>(
    () => ({ session, isAuthenticated: session !== null, isLoading, login, logout, hasModule }),
    [session, isLoading],
  )

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>
}

export function useAuth(): AuthContextValue {
  const ctx = useContext(AuthContext)
  if (!ctx) {
    throw new Error('useAuth must be used within an AuthProvider')
  }
  return ctx
}
