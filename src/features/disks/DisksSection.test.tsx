import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { EditorProvider, useEditor } from '../../app/EditorProvider';
import { BackendProvider } from '../../ipc/context';
import { createMockBackend } from '../../ipc/mock';
import { DisksSection } from './DisksSection';

function Harness() {
  const { state, openSample, setMode } = useEditor();
  return (
    <>
      <button type="button" onClick={() => void openSample()}>
        load
      </button>
      <button type="button" onClick={() => setMode('advanced')}>
        advanced
      </button>
      <span data-testid="disk0">{state.draft?.disks[0] ?? '-'}</span>
      {state.session ? <DisksSection /> : null}
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
  await screen.findByText('Disks');
}

describe('DisksSection', () => {
  it('renders twelve named counts and writes an edit', async () => {
    await setup();
    expect(screen.getByLabelText('Key Chain')).toBeTruthy();
    expect(screen.getByLabelText('HP Disk α')).toBeTruthy();
    expect(screen.getByLabelText('B. Pack')).toBeTruthy();
    expect(screen.getByTestId('disk0').textContent).toBe('0');

    const disk = screen.getByLabelText('HP Disk α');
    await userEvent.clear(disk);
    await userEvent.type(disk, '9{Enter}');
    expect(screen.getByTestId('disk0').textContent).toBe('9');
  });

  it('refuses a second digit so a Normal-mode count cannot exceed 9', async () => {
    await setup();
    const disk = screen.getByLabelText('HP Disk α') as HTMLInputElement;
    expect(disk.maxLength).toBe(1);
    await userEvent.clear(disk);
    await userEvent.type(disk, '99');
    expect(disk.value).toBe('9');
  });

  it('allows the full u16 in Advanced mode', async () => {
    await setup();
    await userEvent.click(screen.getByRole('button', { name: 'advanced' }));
    const disk = screen.getByLabelText('HP Disk α') as HTMLInputElement;
    expect(disk.maxLength).toBe(5);
    await userEvent.clear(disk);
    await userEvent.type(disk, '65535{Enter}');
    await waitFor(() => expect(screen.getByTestId('disk0').textContent).toBe('65535'));
  });
});
