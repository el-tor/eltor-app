import { Routes as RouterRoutes, Route, BrowserRouter } from 'react-router-dom'
import { Layout } from './layout'
import { Connect } from './screens/Connect'
import { Relay } from './screens/Relay'
import { WalletPage } from './screens/WalletPage'

export function Routes() {
  return (
    <BrowserRouter>
      <RouterRoutes>
        <Route path="/" element={<Layout />}>
          <Route index element={<Connect />} />
          <Route path="connect" element={<Connect />} />
          <Route path="relay" element={<Relay />} />
          <Route path="wallet" element={<WalletPage />} />
        </Route>
      </RouterRoutes>
    </BrowserRouter>
  )
}
