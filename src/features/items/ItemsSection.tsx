import { useState } from 'react';
import { ItemRow } from '../../components/ItemRow';
import { SectionCard } from '../../components/SectionCard';
import { useEditor } from '../../app/EditorProvider';
import { errorsByPath } from '../../app/selectors';
import { EMPTY } from '../../lib/items';

const PER_PAGE = 10;
const PAGES = 3;

export function ItemsSection() {
  const { state, setDevice } = useEditor();
  const [page, setPage] = useState(0);
  const { draft, appInfo, mode } = state;
  if (!draft || !appInfo) return null;

  const errors = errorsByPath(state);
  const start = page * PER_PAGE;

  return (
    <SectionCard title="Items">
      <div className="tabs" role="tablist" aria-label="Item pages">
        {Array.from({ length: PAGES }, (_value, index) => (
          <button
            key={index}
            type="button"
            role="tab"
            aria-selected={page === index}
            className={page === index ? 'active' : ''}
            onClick={() => setPage(index)}
          >
            Page {index + 1}
          </button>
        ))}
      </div>
      <div className="item-list">
        {Array.from({ length: PER_PAGE }, (_value, offset) => {
          const slot = start + offset;
          return (
            <ItemRow
              key={slot}
              slot={slot}
              value={draft.device[slot] ?? EMPTY}
              catalogue={appInfo.ui.catalogue}
              mode={mode}
              onChange={(id) => setDevice(slot, id)}
              error={errors.get(`device[${slot}]`) ?? errors.get(`device[${slot}].bonus`)}
            />
          );
        })}
      </div>
    </SectionCard>
  );
}
