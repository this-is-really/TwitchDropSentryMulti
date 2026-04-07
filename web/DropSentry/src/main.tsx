import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import App from './App.tsx'
import { MantineProvider } from '@mantine/core'
import "@mantine/core/styles.css"
import { BrowserRouter } from 'react-router-dom'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <BrowserRouter>
      <MantineProvider withGlobalClasses withCssVariables defaultColorScheme='dark' theme={{ 
        colors: {
          dark: [
              '#f8f9fa',
              '#e9ecef',
              '#dee2e6',
              '#adb5bd',
              '#868e96',
              '#495057',
              '#343a40',
              '#212529',
              '#141619',
              '#0a0c0e',
            ],
        },
        primaryColor: 'blue',
        primaryShade: { light: 6, dark: 5 },
        defaultRadius: 'md',
        fontFamily: "'Fira Code', 'JetBrains Mono', monospace"
       }}>
        <App />
      </MantineProvider>
    </BrowserRouter>
  </StrictMode>,
)
