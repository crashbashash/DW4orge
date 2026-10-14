import { useState } from 'react';
import type { Difficulty, DifficultyChoice, Species } from '../../bindings';
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
  const [species, setSpecies] = useState<Species>('Dorumon');
  const [name, setName] = useState('TST');
  const [story, setStory] = useState<string | null>(null);
  const [difficulty, setDifficulty] = useState<DifficultyChoice>('auto');

  if (!state.appInfo) return null;

  const submit = async () => {
    await newSave({ species, name, story, difficulty });
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

      <p className="muted">
        A difficulty also fills in the easier ones, so the save can be loaded on
        any mode up to the one you pick.
      </p>
    </Modal>
  );
}
