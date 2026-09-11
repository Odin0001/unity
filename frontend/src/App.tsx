import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom'
import { AuthProvider } from './platform/auth/AuthContext'
import { RequireAuth } from './platform/auth/RequireAuth'
import { LoginPage } from './platform/auth/LoginPage'
import { PaywallPage } from './platform/PaywallPage'
import { PaywallNotice } from './platform/PaywallNotice'
import { Dashboard } from './Dashboard'

export default function App() {
  return (
    <BrowserRouter>
      <AuthProvider>
        <Routes>
          <Route path="/login" element={<LoginPage />} />
          <Route path="/paywall" element={<PaywallPage />} />

          <Route element={<RequireAuth />}>
            <Route path="/" element={<Dashboard />} />
            {/* Feature modules register their own routes here once
                modules/inventory and modules/manufacturing land, e.g.:
                <Route path="/inventory/*" element={<InventoryRoutes />} /> */}
          </Route>

          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>

        <PaywallNotice />
      </AuthProvider>
    </BrowserRouter>
  )
}
