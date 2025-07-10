import '@mantine/core/styles.css'
import '@mantine/notifications/styles.css'
import ReactDom from 'react-dom/client'
import React from 'react'
import './globals.css'
import { App } from './App'

ReactDom.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
)
