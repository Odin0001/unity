const STORAGE_KEY = 'tenants.access_token'

/** Wrapped in try/catch because localStorage can throw (private browsing,
 * disabled storage, a full quota) — losing persistence across reloads is
 * an acceptable degradation, throwing out of a token read/write is not. */

export function getAccessToken(): string | null {
  try {
    return window.localStorage.getItem(STORAGE_KEY)
  } catch {
    return null
  }
}

export function setAccessToken(token: string): void {
  try {
    window.localStorage.setItem(STORAGE_KEY, token)
  } catch {
    // Session won't survive a reload; the app still works for this tab.
  }
}

export function clearAccessToken(): void {
  try {
    window.localStorage.removeItem(STORAGE_KEY)
  } catch {
    // See setAccessToken.
  }
}
