import { EditorProvider } from './EditorProvider';
import { AppShell } from './AppShell';

export function App() {
  return (
    <EditorProvider>
      <AppShell />
    </EditorProvider>
  );
}
