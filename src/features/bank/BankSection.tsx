import { useState } from 'react';
import { ItemRow } from '../../components/ItemRow';
import { NumberField } from '../../components/NumberField';
import { SectionCard } from '../../components/SectionCard';
import { useEditor } from '../../app/EditorProvider';
import { errorsByPath } from '../../app/selectors';
import { capFor } from '../../lib/caps';
import { EMPTY } from '../../lib/items';
import { formatNumber } from '../../lib/num';

const PER_PAGE = 24;
const PAGES = 4;

export function BankSection() {
  const { state, setBankItem, setField } = useEditor();
  const [page, setPage] = useState(0);
  const { draft, appInfo, mode } = state;
  if (!draft || !appInfo) return null;

  const errors = errorsByPath(state);
  const start = page * PER_PAGE;
  const balanceCap = capFor(appInfo.ui.caps, 'bank_bit')?.cap.normal_max;

  return (
    <SectionCard title="Bank">
      <div className="grid">
        <NumberField
          label="Bank balance"
          value={draft.bank_bit}
          onChange={(bank_bit) => setField({ bank_bit }, 'bank_bit')}
          error={errors.get('bank_bit')}
          hint={mode === 'normal' && balanceCap ? `max ${formatNumber(balanceCap)}` : undefined}
        />
      </div>

      <div className="tabs" role="tablist" aria-label="Bank pages">
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
              value={draft.bank_items[slot] ?? EMPTY}
              catalogue={appInfo.ui.catalogue}
              mode={mode}
              onChange={(id) => setBankItem(slot, id)}
              error={errors.get(`bank_items[${slot}]`) ?? errors.get(`bank_items[${slot}].bonus`)}
            />
          );
        })}
      </div>
    </SectionCard>
  );
}
