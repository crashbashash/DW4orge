import { useState } from 'react';
import { SectionCard } from '../../components/SectionCard';
import { SelectField } from '../../components/SelectField';
import { useEditor } from '../../app/EditorProvider';
import { diffStory, resolveDifficulty } from '../../app/store';
import { applyPresetToStory, mirrorPreview, storyGroups } from '../../lib/story';

const DIFFICULTIES = ['Normal', 'Hard', 'VeryHard'] as const;

const DIFFICULTY_LABELS = {
  Normal: 'Normal',
  Hard: 'Hard',
  VeryHard: 'Very Hard',
} as const;

const DIFFICULTY_OPTIONS = [
  { value: 'auto', label: 'Auto (detected)' },
  ...DIFFICULTIES.map((difficulty) => ({
    value: difficulty as string,
    label: DIFFICULTY_LABELS[difficulty],
  })),
];

export function StorySection() {
  const { state, setStory, setStoryDraft, setDifficulty } = useEditor();
  // The preset control is an action, not a stored value, so it resets itself.
  const [preset, setPreset] = useState('');
  const { draft, appInfo, session } = state;
  if (!draft || !appInfo || !session) return null;

  const groups = storyGroups(appInfo.ui.flag_labels);
  const governed = groups.flatMap((group) => group.flags.map((flag) => flag.flag));

  // Each difficulty keeps its own draft, so this is only the one on screen;
  // the others stay in the store untouched until their turn comes.
  const activeDifficulty = resolveDifficulty(state.difficulty, session);
  const story = state.stories[activeDifficulty];
  const pending = diffStory(story, session.baselines[activeDifficulty], activeDifficulty);
  const preview = mirrorPreview(pending, appInfo.ui.mirrors);

  const difficultyValue = state.difficulty === 'auto' ? 'auto' : state.difficulty.fixed;

  return (
    <SectionCard title="Story">
      <div className="grid">
        <SelectField
          label={`Difficulty (detected: ${DIFFICULTY_LABELS[session.view.difficulty]})`}
          value={difficultyValue}
          options={DIFFICULTY_OPTIONS}
          onChange={(value) => {
            setDifficulty(
              value === 'auto' ? 'auto' : { fixed: value as (typeof DIFFICULTIES)[number] },
            );
          }}
        />
        <SelectField
          label="Preset"
          value={preset}
          placeholder="Apply preset…"
          options={appInfo.ui.story_presets.map((entry) => ({
            value: entry.name,
            label: entry.name,
          }))}
          onChange={(name) => {
            const found = appInfo.ui.story_presets.find((entry) => entry.name === name);
            if (found) setStoryDraft(applyPresetToStory(found, story, governed));
            setPreset('');
          }}
        />
      </div>

      <p className="muted">
        Story is stored per difficulty, so these checkboxes show{' '}
        {DIFFICULTY_LABELS[activeDifficulty]}&apos;s flags. Saving a difficulty also
        fills in the easier ones, so a harder mode stays usable.
      </p>

      {groups.map((group) => (
        <fieldset key={group.key}>
          <legend>{group.label}</legend>
          <div className="checkbox-grid">
            {group.flags.map((flag) => (
              <label key={flag.flag} className="check">
                <input
                  type="checkbox"
                  checked={story.flags[flag.flag] ?? false}
                  onChange={(event) => setStory('flag', flag.flag, event.target.checked)}
                />
                <span>{flag.label}</span>
              </label>
            ))}
          </div>
        </fieldset>
      ))}

      <fieldset>
        <legend>Folders</legend>
        <div className="checkbox-grid">
          {appInfo.ui.folder_labels.map((label, index) => (
            <label key={label} className="check">
              <input
                type="checkbox"
                checked={story.folders[index] ?? false}
                onChange={(event) => setStory('folder', index, event.target.checked)}
              />
              <span>{label}</span>
            </label>
          ))}
        </div>
      </fieldset>

      <details className="card mirror-writes">
        <summary>Mirror writes ({DIFFICULTY_LABELS[activeDifficulty]})</summary>
        <p className="muted mirror-note">
          The game restores story flags from these backup “mirror” flags when it loads a save, so an
          edit only sticks if the mirror is written too. These are the mirrors your current changes
          will write.
        </p>
        {preview.length === 0 ? (
          <p className="muted">No story changes yet.</p>
        ) : (
          <ul className="mirror-preview">
            {preview.map((row, index) => (
              <li key={`${row.difficulty}-${row.flag}-${index}`}>
                {DIFFICULTY_LABELS[row.difficulty]} · flag {row.flag} ={' '}
                <span className={row.value ? 'mirror-set' : 'mirror-clear'}>
                  {row.value ? 'set' : 'cleared'}
                </span>
              </li>
            ))}
          </ul>
        )}
      </details>
    </SectionCard>
  );
}
