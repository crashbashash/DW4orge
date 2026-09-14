import { useState } from 'react';
import type { Difficulty, DifficultyChoice, Species } from '../../bindings';
import { Modal } from '../../components/Modal';
import { SelectField } from '../../components/SelectField';
import { TextField } from '../../components/TextField';
import { useEditor } from '../../app/EditorProvider';
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
        <TextField label="Player name" value={name} maxLength={3} onChange={setName} />

        <div className="field">
          <label className="label" htmlFor="new-story">
            Story preset
          </label>
          <select
            id="new-story"
            value={story ?? ''}
            onChange={(event) => setStory(event.target.value || null)}
          >
            <option value="">None (new game)</option>
            {state.appInfo.ui.story_presets.map((preset) => (
              <option key={preset.name} value={preset.name}>
                {preset.name}
              </option>
            ))}
          </select>
        </div>

        <div className="field">
          <label className="label" htmlFor="new-difficulty">
            Difficulty
          </label>
          <select
            id="new-difficulty"
            value={difficulty === 'auto' ? 'auto' : difficulty.fixed}
            onChange={(event) => {
              const value = event.target.value;
              setDifficulty(value === 'auto' ? 'auto' : { fixed: value as Difficulty });
            }}
          >
            <option value="auto">Auto (Normal)</option>
            {DIFFICULTIES.map((entry) => (
              <option key={entry} value={entry}>
                {DIFFICULTY_LABELS[entry]}
              </option>
            ))}
          </select>
        </div>
      </div>
    </Modal>
  );
}
