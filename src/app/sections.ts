import type { SectionId } from './store';

/** The six editor sections, in sidebar order (spec §7.2). */
export const SECTIONS: readonly { id: SectionId; label: string }[] = [
  { id: 'character', label: 'Character' },
  { id: 'items', label: 'Items' },
  { id: 'equipment', label: 'Equipment' },
  { id: 'disks', label: 'Disks' },
  { id: 'story', label: 'Story' },
  { id: 'bank', label: 'Bank' },
];
