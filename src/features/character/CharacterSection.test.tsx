import type { SpeciesStats } from '../../bindings';
import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { EditorProvider, useEditor } from '../../app/EditorProvider';
import { BackendProvider } from '../../ipc/context';
import { createMockBackend, type MockOptions } from '../../ipc/mock';
import { CharacterSection } from './CharacterSection';

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
      {state.session ? <CharacterSection /> : null}
    </>
  );
}

async function setup(options: MockOptions = {}) {
  render(
    <BackendProvider backend={createMockBackend(options)}>
      <EditorProvider>
        <Harness />
      </EditorProvider>
    </BackendProvider>,
  );
  await userEvent.click(screen.getByRole('button', { name: 'load' }));
  await screen.findByText('Character');
}

describe('CharacterSection', () => {
  it('reloads the target species stats on a species switch', async () => {
    await setup({
      speciesStats: () => ({
        level: 42,
        exp: 5000,
        tech: [2, 2, 2, 2, 2, 2, 2, 2, 2],
        upcnt: [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
      }),
    });

    expect((screen.getByLabelText('Level') as HTMLInputElement).value).toBe('1');
    await userEvent.click(screen.getByRole('button', { name: /species/i }));
    await userEvent.click(await screen.findByRole('option', { name: 'Agumon' }));

    await waitFor(() =>
      expect((screen.getByLabelText('Level') as HTMLInputElement).value).toBe('42'),
    );
    expect((screen.getByLabelText('EXP') as HTMLInputElement).value).toBe('5000');
  });

  it('keeps unsaved progression when a species switch is cancelled', async () => {
    await setup();
    await userEvent.clear(screen.getByLabelText('Level'));
    await userEvent.type(screen.getByLabelText('Level'), '17{Enter}');
    await userEvent.click(screen.getByRole('button', { name: /species/i }));
    await userEvent.click(await screen.findByRole('option', { name: 'Agumon' }));
    expect(await screen.findByRole('dialog', { name: /discard progression/i })).toBeTruthy();
    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    expect((screen.getByLabelText('Level') as HTMLInputElement).value).toBe('17');
    expect(screen.getByRole('button', { name: /species/i }).textContent).toContain('Dorumon');
  });

  it('switches only after explicitly discarding unsaved progression', async () => {
    await setup();
    await userEvent.clear(screen.getByLabelText('Level'));
    await userEvent.type(screen.getByLabelText('Level'), '17{Enter}');
    await userEvent.click(screen.getByRole('button', { name: /species/i }));
    await userEvent.click(await screen.findByRole('option', { name: 'Agumon' }));
    await userEvent.click(await screen.findByRole('button', { name: 'Discard and switch' }));
    expect((screen.getByLabelText('Level') as HTMLInputElement).value).toBe('1');
    expect(screen.getByRole('button', { name: /species/i }).textContent).toContain('Agumon');
  });

  it('guards progression edited while a species query is in flight', async () => {
    let resolve: ((value: SpeciesStats) => void) | undefined;
    await setup({ speciesStats: () => new Promise<SpeciesStats>((done) => { resolve = done; }) });
    await userEvent.click(screen.getByRole('button', { name: /species/i }));
    await userEvent.click(await screen.findByRole('option', { name: 'Agumon' }));
    await userEvent.clear(screen.getByLabelText('Level'));
    await userEvent.type(screen.getByLabelText('Level'), '17{Enter}');
    await act(async () => { resolve?.({ level: 1, exp: 0,
      tech: [1, 1, 1, 1, 1, 1, 1, 1, 1], upcnt: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0] }); });
    expect(await screen.findByRole('dialog', { name: /discard progression/i })).toBeTruthy();
    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    expect((screen.getByLabelText('Level') as HTMLInputElement).value).toBe('17');
  });

  it('reports a species read failure and keeps the current draft', async () => {
    await setup({ speciesStats: async () => { throw { kind: 'unsupported', message: 'read failed' }; } });
    await userEvent.click(screen.getByRole('button', { name: /species/i }));
    await userEvent.click(await screen.findByRole('option', { name: 'Agumon' }));
    expect((await screen.findByRole('alert')).textContent).toContain('read failed');
    expect(screen.getByRole('button', { name: /species/i }).textContent).toContain('Dorumon');
  });

  it('syncs EXP to the level threshold', async () => {
    await setup();
    const level = screen.getByLabelText('Level');
    await userEvent.clear(level);
    await userEvent.type(level, '2{Enter}');
    await waitFor(() =>
      expect((screen.getByLabelText('EXP') as HTMLInputElement).value).toBe('341'),
    );
  });

  it('limits the player name to eight characters', async () => {
    await setup();
    const name = screen.getByLabelText('Player name');
    await userEvent.clear(name);
    await userEvent.type(name, 'abcdefghij');
    expect((name as HTMLInputElement).value).toBe('abcdefgh');
  });

  it('caps a value box at the Normal-mode maximum', async () => {
    await setup();
    const bit = screen.getByLabelText('BIT') as HTMLInputElement;
    await userEvent.clear(bit);
    await userEvent.type(bit, '99999999{Enter}');
    await waitFor(() => expect(bit.value).toBe('9999999'));
  });

  it('stops Level at three digits and X-Data at four', async () => {
    await setup();
    const level = screen.getByLabelText('Level') as HTMLInputElement;
    await userEvent.clear(level);
    await userEvent.type(level, '1234{Enter}');
    await waitFor(() => expect(level.value).toBe('123'));

    const xdata = screen.getByLabelText('X-Data') as HTMLInputElement;
    await userEvent.clear(xdata);
    await userEvent.type(xdata, '99999{Enter}');
    await waitFor(() => expect(xdata.value).toBe('9999'));
  });

  it('caps HP/MP max at five digits and other power-ups at four', async () => {
    await setup();
    const hp = screen.getByLabelText('HP max') as HTMLInputElement;
    await userEvent.clear(hp);
    await userEvent.type(hp, '999999{Enter}');
    await waitFor(() => expect(hp.value).toBe('99999'));

    const strength = screen.getByLabelText('Strength') as HTMLInputElement;
    await userEvent.clear(strength);
    await userEvent.type(strength, '99999{Enter}');
    await waitFor(() => expect(strength.value).toBe('9999'));
  });

  it('leaves the value box uncapped in Advanced mode', async () => {
    await setup();
    await userEvent.click(screen.getByRole('button', { name: 'advanced' }));
    const bit = screen.getByLabelText('BIT') as HTMLInputElement;
    await userEvent.clear(bit);
    await userEvent.type(bit, '99999999{Enter}');
    await waitFor(() => expect(bit.value).toBe('99999999'));
  });

  it('labels the nine techniques with the codes::TECHNIQUES names', async () => {
    await setup();
    for (const name of ['blunt', 'slash', 'stab', 'bash', 'shot', 'crush', 'blast', 'heal', 'force']) {
      expect(screen.getByLabelText(name)).toBeTruthy();
    }
  });
});
