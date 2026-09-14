import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { App } from './app/App';
import { applyTheme, readStoredTheme, resolveTheme } from './app/theme';
import './styles/tokens.css';
import './styles/global.css';

const prefersDark = window.matchMedia('(prefers-color-scheme: dark)');
applyTheme(resolveTheme(readStoredTheme(), prefersDark.matches), document.documentElement);
prefersDark.addEventListener('change', (event) => {
  // An explicit choice wins; only follow the OS while none is stored.
  if (readStoredTheme() === null) {
    applyTheme(resolveTheme(null, event.matches), document.documentElement);
  }
});

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
