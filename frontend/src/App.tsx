import '@mantine/core/styles.css'
import { Routes } from './routes'
import './globals.css'
import { MantineProvider } from '@mantine/core'
import { Provider } from 'react-redux'
import { store, persistor } from './store'
import { PersistGate } from 'redux-persist/integration/react'
import { theme } from './theme'

export function App() {
  return (
    <Provider store={store}>
      <PersistGate loading={null} persistor={persistor}>
        <MantineProvider theme={theme} forceColorScheme="dark">
          <Routes />
        </MantineProvider>
      </PersistGate>
    </Provider>
  )
}