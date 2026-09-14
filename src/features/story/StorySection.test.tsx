import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { EditorProvider, useEditor } from '../../app/EditorProvider';
import { BackendProvider } from '../../ipc/context';
import { createMockBackend } from '../../ipc/mock';
import { StorySection } from './StorySection';

function Harness() {
  const { state, openSample } = useEditor();
  return (
    <>
      <button type="button" onClick={() => void openSample()}>
        load
      </button>
      <span data-testid="difficulty">
        {typeof state.difficulty === 'string' ? state.difficulty : state.difficulty.fixed}
      </span>
      {state.session ? <StorySection /> : null}
    </>
  );
}

async function setup() {
  render(
    <BackendProvider backend={createMockBackend()}>
      <EditorProvider>
        <Harness />
      </EditorProvider>
    </BackendProvider>,
  );
  await userEvent.click(screen.getByRole('button', { name: 'load' }));
  await screen.findByText('Story');
}

describe('StorySection', () => {
  it('applies a preset by replacing the governed flags', async () => {
    await setup();
    const tutorial = screen.getByLabelText(/tutorial \(fresh start\)/i) as HTMLInputElement;
    // The real save has flag 1 (the tutorial marker) set.
    expect(tutorial.checked).toBe(true);

    await userEvent.click(screen.getByRole('button', { name: /preset/i }));
    await userEvent.click(await screen.findByRole('option', { name: 'After World 1' }));

    expect((screen.getByLabelText(/tutorial \(fresh start\)/i) as HTMLInputElement).checked).toBe(
      false,
    );
    expect((screen.getByLabelText(/world 1 access/i) as HTMLInputElement).checked).toBe(true);
  });

  it('overrides the detected difficulty', async () => {
    await setup();
    expect(screen.getByTestId('difficulty').textContent).toBe('Normal');
    await userEvent.click(screen.getByRole('button', { name: /difficulty/i }));
    await userEvent.click(await screen.findByRole('option', { name: 'Hard' }));
    expect(screen.getByTestId('difficulty').textContent).toBe('Hard');
  });

  it('previews the mirror a pending flag edit writes', async () => {
    await setup();
    await userEvent.click(screen.getByLabelText(/apocalymon/i));
    // Active flag 66 mirrors to 707 on Normal.
    expect(screen.getByText('flag 707 ← 1')).toBeTruthy();
  });
});
