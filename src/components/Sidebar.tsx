import { useEditor } from '../app/EditorProvider';
import { SECTIONS } from '../app/sections';

export function Sidebar() {
  const { state, setSection } = useEditor();

  return (
    <nav className="sidebar" aria-label="Sections">
      {SECTIONS.map((section) => (
        <button
          key={section.id}
          type="button"
          className={state.section === section.id ? 'nav-button active' : 'nav-button'}
          aria-current={state.section === section.id ? 'page' : undefined}
          disabled={!state.session}
          onClick={() => setSection(section.id)}
        >
          {section.label}
        </button>
      ))}
    </nav>
  );
}
