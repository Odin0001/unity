import { clearAccessToken, getAccessToken } from '../platform/auth/tokenStorage'

/** All backend routes live under this versioned, absolute prefix. Dev
 * requests are proxied to the Axum backend (see vite.config.ts); in
 * production the built SPA is served by the same process that owns this
 * prefix, so it resolves same-origin with no base URL needed. */
export const API_BASE = '/api/v1'

/** Fired on the window whenever a request comes back 403 Forbidden, so any
 * part of the UI can show a paywall notice without api/client.ts knowing
 * about a specific component. See platform/PaywallNotice.tsx. */
export const PAYWALL_EVENT = 'tenants:paywall'

export interface PaywallEventDetail {
  path: string
  message: string
}

export class ApiError extends Error {
  readonly status: number
  readonly body: unknown

  constructor(message: string, status: number, body?: unknown) {
    super(message)
    this.name = 'ApiError'
    this.status = status
    this.body = body
  }
}

export interface RequestOptions extends Omit<RequestInit, 'body'> {
  body?: unknown
  /** Skip attaching the Authorization header — for endpoints like login
   * that are called before a token exists. */
  skipAuth?: boolean
}

/** Shape every error response from the backend uses, e.g.
 * `{"error": "module 'manufacturing' is not included in this tenant's
 * current plan"}` from platform::feature_flags::FeatureFlagError. */
interface BackendErrorBody {
  error?: string
}

async function safeJson(response: Response): Promise<unknown> {
  const text = await response.text()
  if (!text) return undefined
  try {
    return JSON.parse(text) as unknown
  } catch {
    return text
  }
}

function extractErrorMessage(body: unknown, fallback: string): string {
  if (body && typeof body === 'object' && 'error' in body) {
    const message = (body as BackendErrorBody).error
    if (typeof message === 'string' && message.length > 0) return message
  }
  return fallback
}

async function request<T>(path: string, options: RequestOptions = {}): Promise<T> {
  const { body, skipAuth, headers, ...rest } = options

  const finalHeaders = new Headers(headers)
  finalHeaders.set('Accept', 'application/json')
  if (body !== undefined) finalHeaders.set('Content-Type', 'application/json')

  if (!skipAuth) {
    const token = getAccessToken()
    if (token) finalHeaders.set('Authorization', `Bearer ${token}`)
  }

  const response = await fetch(`${API_BASE}${path}`, {
    ...rest,
    headers: finalHeaders,
    body: body !== undefined ? JSON.stringify(body) : undefined,
  })

  // Global 401 handling: the opaque token is gone or expired server-side.
  // There's nothing a caller can usefully do but re-authenticate, so this
  // clears the stale token and redirects to /login once, here, instead of
  // every call site having to remember to check for 401.
  if (response.status === 401) {
    clearAccessToken()
    if (typeof window !== 'undefined' && !window.location.pathname.startsWith('/login')) {
      const redirect = encodeURIComponent(window.location.pathname + window.location.search)
      window.location.assign(`/login?redirect=${redirect}`)
    }
    throw new ApiError('Session expired, please sign in again.', 401)
  }

  // Global 403 handling: authenticated, but the tenant's plan doesn't
  // include this module (platform::feature_flags::RequireModule on the
  // backend). Broadcast it so any mounted paywall notice can react,
  // without api/client.ts needing to know what's currently on screen.
  if (response.status === 403) {
    const responseBody = await safeJson(response)
    const message = extractErrorMessage(responseBody, 'This feature is not available on your current plan.')

    if (typeof window !== 'undefined') {
      window.dispatchEvent(
        new CustomEvent<PaywallEventDetail>(PAYWALL_EVENT, { detail: { path, message } }),
      )
    }

    throw new ApiError(message, 403, responseBody)
  }

  if (!response.ok) {
    const responseBody = await safeJson(response)
    throw new ApiError(extractErrorMessage(responseBody, response.statusText), response.status, responseBody)
  }

  if (response.status === 204) return undefined as T
  return (await safeJson(response)) as T
}

export const apiClient = {
  get: <T>(path: string, options?: RequestOptions) => request<T>(path, { ...options, method: 'GET' }),
  post: <T>(path: string, body?: unknown, options?: RequestOptions) =>
    request<T>(path, { ...options, method: 'POST', body }),
  patch: <T>(path: string, body?: unknown, options?: RequestOptions) =>
    request<T>(path, { ...options, method: 'PATCH', body }),
  put: <T>(path: string, body?: unknown, options?: RequestOptions) =>
    request<T>(path, { ...options, method: 'PUT', body }),
  delete: <T>(path: string, options?: RequestOptions) => request<T>(path, { ...options, method: 'DELETE' }),
}
