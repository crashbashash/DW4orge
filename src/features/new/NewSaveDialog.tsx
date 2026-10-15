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

export function NewSaveDialog({ isOpen, onClose, boxCreation = false }: { isOpen: boolean; onClose: () => void; boxCreation?: boolean }) {
  const { state, newSave, createBox } = useEditor();
  const [boxNumber, setBoxNumber] = useState(0);
  const emptyBoxes = state.session?.empty_boxes ?? [];
  const selectedBox = emptyBoxes.includes(boxNumber) ? boxNumber : emptyBoxes[0];
  const [format, setFormat] = useState<SourceKind>('memcard');
  const [region, setRegion] = useState<Region>('usa');
  const [species, setSpecies] = useState<Species>('Dorumon');
  const [name, setName] = useState('TST');
  const [story, setStory] = useState<string | null>(null);
  const [difficulty, setDifficulty] = useState<DifficultyChoice>('auto');

  if (!state.appInfo) return null;

  const submit = async () => {
    const request = { format: boxCreation ? undefined : format, region: boxCreation ? state.session?.region ?? null : region, species, name, story, difficulty };
    const ok = boxCreation ? selectedBox !== undefined && await createBox(selectedBox, request) : await newSave(request);
    if (ok) onClose();
  };

  return (
    <Modal
      title={boxCreation ? "Create character box" : "New save"}
      isOpen={isOpen}
      onClose={onClose}
      footer={
        <>
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button type="button" disabled={state.status === 'busy' || (boxCreation && selectedBox === undefined)} onClick={() => void submit()}>
            Create
          </button>
        </>
      }
    >
      <div className="grid">
        {boxCreation ? <SelectField label="Empty box" value={String(selectedBox ?? '')}
          options={emptyBoxes.map(number => ({ value: String(number), label: `Box ${number} (empty)` }))}
          onChange={value => setBoxNumber(Number(value))} /> : <>
        <SelectField label="Save format" value={format} options={[
          { value: 'memcard', label: 'PS2 memory card (.ps2)' },
          { value: 'gamecube', label: 'GameCube save (.gci)' },
          { value: 'gamecube_card', label: 'GameCube memory card (.raw, 251 blocks)' },
        ]} onChange={setFormat} />
        <SelectField label="Region" value={region} options={[
          { value: 'usa', label: 'USA' }, { value: 'europe', label: 'Europe' }, { value: 'japan', label: 'Japan (Digimon World X)' },
        ]} onChange={setRegion} />
        </>}
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

      {state.error && <p role="alert" className="danger">{state.error}</p>}
      {boxCreation && <p className="muted">Creates a new character in the opened save. Other boxes are preserved.{state.session?.source === 'xbox_mu' ? ' On memory units, this uses the save container selected in Character box.' : ''} Use Save or Save As to write it.</p>}
      {!boxCreation && format !== 'memcard' && <p className="muted">
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
