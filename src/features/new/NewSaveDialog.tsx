import { useState } from 'react';
import type { Difficulty, DifficultyChoice, Region, SourceKind, Species } from '../../bindings';
import { Modal } from '../../components/Modal';
import { SelectField } from '../../components/SelectField';
import { TextField } from '../../components/TextField';
import { useEditor } from '../../app/EditorProvider';
import { PLAYER_NAME_MAX } from '../../lib/name';
import { SPECIES } from '../../lib/species';

const DIFFICULTIES: readonly Difficulty[] = ['Normal', 'Hard', 'VeryHard'];

const DIFFICULTY_LABELS: Record<Difficulty, string> = {
  Normal: 'Normal',
  Hard: 'Hard',
  VeryHard: 'Very Hard',
};

export function NewSaveDialog({ isOpen, onClose }: { isOpen: boolean; onClose: () => void }) {
  const { state, newSave } = useEditor();
  const [format, setFormat] = useState<SourceKind>('memcard');
  const [region, setRegion] = useState<Region>('usa');
  const [species, setSpecies] = useState<Species>('Dorumon');
  const [name, setName] = useState('TST');
  const [story, setStory] = useState<string | null>(null);
  const [difficulty, setDifficulty] = useState<DifficultyChoice>('auto');

  if (!state.appInfo) return null;

  const submit = async () => {
    await newSave({ format, region, species, name, story, difficulty });
    onClose();
  };

  return (
    <Modal
      title="New save"
      isOpen={isOpen}
      onClose={onClose}
      footer={
        <>
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button type="button" onClick={() => void submit()}>
            Create
          </button>
        </>
      }
    >
      <div className="grid">
        <SelectField label="Save format" value={format} options={[
          { value: 'memcard', label: 'PS2 memory card (.ps2)' },
          { value: 'gamecube', label: 'GameCube save (.gci)' },
          { value: 'gamecube_card', label: 'GameCube memory card (.raw, 251 blocks)' },
        ]} onChange={setFormat} />
        <SelectField label="Region" value={region} options={[
          { value: 'usa', label: 'USA' }, { value: 'europe', label: 'Europe' }, { value: 'japan', label: 'Japan (Digimon World X)' },
        ]} onChange={setRegion} />
        <SelectField
          label="Species"
          value={species}
          options={SPECIES.map((entry) => ({ value: entry, label: entry }))}
          onChange={setSpecies}
        />
        <TextField label="Player name" value={name} maxLength={PLAYER_NAME_MAX} onChange={setName} />

        <SelectField
          label="Story preset"
          value={story ?? ''}
          options={[
            { value: '', label: 'None (new game)' },
            ...state.appInfo.ui.story_presets.map((preset) => ({
              value: preset.name,
              label: preset.name,
            })),
          ]}
          onChange={(value) => setStory(value || null)}
        />

        <SelectField
          label="Difficulty"
          value={difficulty === 'auto' ? 'auto' : difficulty.fixed}
          options={[
            { value: 'auto', label: 'Auto (Normal)' },
            ...DIFFICULTIES.map((entry) => ({
              value: entry as string,
              label: DIFFICULTY_LABELS[entry],
            })),
          ]}
          onChange={(value) => {
            setDifficulty(value === 'auto' ? 'auto' : { fixed: value as Difficulty });
          }}
        />
      </div>

      {format !== 'memcard' && <p className="muted">
        Creates a new GameCube save with a blank banner and icon. The game replaces them
        when it saves. GCI files can be imported into Dolphin; full cards can be mounted directly.
      </p>}
      <p className="muted">
        Story flags are written to the selected difficulty and the easier ones.
        The game still checks completion to decide which modes are available
        when you load the save.
      </p>
    </Modal>
  );
}
