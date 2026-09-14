import type { EditSet } from '../../bindings';
import { NumberField } from '../../components/NumberField';
import { SectionCard } from '../../components/SectionCard';
import { useEditor } from '../../app/EditorProvider';
import { errorsByPath } from '../../app/selectors';
import { capFor } from '../../lib/caps';
import { DISK_LABELS } from '../../lib/disks';
import { formatNumber } from '../../lib/num';

function setDisk(disks: EditSet['disks'], index: number, value: number): EditSet['disks'] {
  const next: EditSet['disks'] = [
    disks[0],
    disks[1],
    disks[2],
    disks[3],
    disks[4],
    disks[5],
    disks[6],
    disks[7],
    disks[8],
    disks[9],
    disks[10],
    disks[11],
  ];
  next[index] = value;
  return next;
}

export function DisksSection() {
  const { state, setField } = useEditor();
  const { draft, appInfo } = state;
  if (!draft || !appInfo) return null;

  const errors = errorsByPath(state);
  // The field is a `u16`, so 65,535 is the data-type limit in both modes, not
  // just a Normal-mode cap. Capping here stops serde rejecting an overlarge
  // number after the editor has already accepted it.
  const diskMax = capFor(appInfo.ui.caps, 'disks')?.cap.normal_max;

  return (
    <SectionCard title="Disks">
      <p className="muted">Owned disk counts, 0–{formatNumber(diskMax ?? 65_535)}.</p>
      <div className="grid preserve-case">
        {draft.disks.map((count, index) => (
          <NumberField
            key={index}
            label={DISK_LABELS[index] ?? `Disk ${index + 1}`}
            value={count}
            onChange={(value) => setField({ disks: setDisk(draft.disks, index, value) }, `disks[${index}]`)}
            error={errors.get(`disks[${index}]`)}
            max={diskMax}
          />
        ))}
      </div>
    </SectionCard>
  );
}
