import { Routes } from './routes'
import './globals.css'
import { MantineProvider } from '@mantine/core'
import { Notifications } from '@mantine/notifications'
import { Provider } from 'react-redux'
import { store, persistor } from './store'
import { PersistGate } from 'redux-persist/integration/react'
import { theme } from './theme'
import { useGlobalLogStream } from './hooks/useGlobalLogStream'

function AppContent() {
  // Initialize global log streaming service
  useGlobalLogStream()
  
  return <Routes />
}

export function App() {
  return (
    <Provider store={store}>
      <PersistGate loading={null} persistor={persistor}>
        <MantineProvider theme={theme} forceColorScheme="dark">
          <Notifications />
          <AppContent />
        </MantineProvider>
      </PersistGate>
    </Provider>
  )
}