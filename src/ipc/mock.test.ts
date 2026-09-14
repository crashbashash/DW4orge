import { describe, expect, it } from 'vitest';
import { emptyEditSet } from '../lib/editSet';
import { createMockBackend } from './mock';

describe('createMockBackend', () => {
  it('serves a real catalogue and save', async () => {
    const backend = createMockBackend();
    const info = await backend.appInfo();
    expect(info.ui.catalogue).toHaveLength(539);
    expect(info.schema_version).toBe(2);
    const opened = await backend.openSave('/mock/x.raw');
    expect(opened.source).toBe('raw');
    expect(opened.view.species).toBe('Dorumon');
  });

  it('raises no_open_document before an open', async () => {
    const backend = createMockBackend();
    await expect(backend.getView('normal')).rejects.toMatchObject({ kind: 'no_open_document' });
  });

  it('lets a test inject a validation report', async () => {
    const backend = createMockBackend({
      validateEdits: () => ({
        errors: [{ path: 'bit', message: 'too big', severity: 'error' }],
        warnings: [],
      }),
    });
    await backend.openSave('/mock/x.raw');
    const edits = { ...emptyEditSet(), bit: 10_000_000 };
    const report = await backend.validateEdits(edits, 'normal');
    expect(report.errors[0].path).toBe('bit');
  });

  it('reflects an applied draft in the next view', async () => {
    const backend = createMockBackend();
    await backend.openSave('/mock/x.raw');
    const edits = { ...emptyEditSet(), bit: 1234 };
    const saved = await backend.save(edits, 'normal');
    expect(saved.view.bit).toBe(1234);
    expect((await backend.getView('normal')).bit).toBe(1234);
  });

  it('hands out the queued dialog paths once each', async () => {
    const backend = createMockBackend({ openPaths: ['/a.ps2', null], savePaths: ['/b.raw'] });
    expect(await backend.pickOpenPath()).toBe('/a.ps2');
    expect(await backend.pickOpenPath()).toBeNull();
    expect(await backend.pickSavePath('out.raw')).toBe('/b.raw');
  });
});
