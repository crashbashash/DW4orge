import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { App } from './app/App';
import { applyTheme, detectTheme, readStoredTheme } from './app/theme';
import { BackendProvider } from './ipc/context';
import { createMockBackend } from './ipc/mock';
import { tauriBackend } from './ipc/tauri';
import './styles/tokens.css';
import './styles/global.css';

// The real shell defines `__TAURI_INTERNALS__`; a plain browser does not, so it
// gets the fixture-backed mock and the whole UI still runs.
const backend = '__TAURI_INTERNALS__' in window ? tauriBackend : createMockBackend();

applyTheme(detectTheme(), document.documentElement);
window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
  // An explicit choice wins; only follow the OS while none is stored.
  if (readStoredTheme() === null) applyTheme(detectTheme(), document.documentElement);
});

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <BackendProvider backend={backend}>
      <App />
    </BackendProvider>
  </StrictMode>,
);
