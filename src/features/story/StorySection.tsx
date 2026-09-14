import { SectionCard } from '../../components/SectionCard';
import { useEditor } from '../../app/EditorProvider';
import { diffStory } from '../../app/store';
import { applyPresetToStory, mirrorPreview, storyGroups } from '../../lib/story';

const DIFFICULTIES = ['Normal', 'Hard', 'VeryHard'] as const;

const DIFFICULTY_LABELS = {
  Normal: 'Normal',
  Hard: 'Hard',
  VeryHard: 'Very Hard',
} as const;

export function StorySection() {
  const { state, setStory, setStoryDraft, setDifficulty } = useEditor();
  const { draft, story, appInfo, session } = state;
  if (!draft || !story || !appInfo || !session) return null;

  const groups = storyGroups(appInfo.ui.flag_labels);
  const governed = groups.flatMap((group) => group.flags.map((flag) => flag.flag));

  const pending = diffStory(story, session.baselineStory);
  const activeDifficulty =
    state.difficulty === 'auto' ? session.view.difficulty : state.difficulty.fixed;
  const preview = mirrorPreview(pending, appInfo.ui.mirrors, activeDifficulty);

  const difficultyValue = state.difficulty === 'auto' ? 'auto' : state.difficulty.fixed;

  return (
    <SectionCard title="Story">
      <div className="grid">
        <div className="field">
          <label className="label" htmlFor="story-difficulty">
            Difficulty (detected: {DIFFICULTY_LABELS[session.view.difficulty]})
          </label>
          <select
            id="story-difficulty"
            value={difficultyValue}
            onChange={(event) => {
              const value = event.target.value;
              setDifficulty(value === 'auto' ? 'auto' : { fixed: value as (typeof DIFFICULTIES)[number] });
            }}
          >
            <option value="auto">Auto (detected)</option>
            {DIFFICULTIES.map((difficulty) => (
              <option key={difficulty} value={difficulty}>
                {DIFFICULTY_LABELS[difficulty]}
              </option>
            ))}
          </select>
        </div>

        <div className="field">
          <label className="label" htmlFor="story-preset">
            Preset
          </label>
          <select
            id="story-preset"
            defaultValue=""
            onChange={(event) => {
              const name = event.target.value;
              if (!name) return;
              const preset = appInfo.ui.story_presets.find((entry) => entry.name === name);
              if (preset) setStoryDraft(applyPresetToStory(preset, story, governed));
              event.target.value = '';
            }}
          >
            <option value="">Apply preset…</option>
            {appInfo.ui.story_presets.map((preset) => (
              <option key={preset.name} value={preset.name}>
                {preset.name}
              </option>
            ))}
          </select>
        </div>
      </div>

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

      <div className="card">
        <h3>Mirror preview ({DIFFICULTY_LABELS[activeDifficulty]})</h3>
        {preview.length === 0 ? (
          <p className="muted">No story changes yet.</p>
        ) : (
          <ul className="mirror-preview">
            {preview.map((row, index) => (
              <li key={`${row.flag}-${index}`}>
                flag {row.flag} ← {row.value ? '1' : '0'}
              </li>
            ))}
          </ul>
        )}
      </div>
    </SectionCard>
  );
}
