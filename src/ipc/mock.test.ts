import { describe, expect, it } from 'vitest';
import { emptyEditSet } from '../lib/editSet';
import { viewToEditSet } from '../app/store';
import { createMockBackend } from './mock';

describe('createMockBackend', () => {
  it('serves a real catalogue and save', async () => {
    const backend = createMockBackend();
    const info = await backend.appInfo();
    expect(info.ui.catalogue).toHaveLength(539);
    expect(info.schema_version).toBe(7);
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


it('keeps saved created boxes when switching, and discards only unsaved creations', async () => {
  const backend = createMockBackend();
  const first = await backend.openSave('/mock/source.ps2');
  const request = { format: 'memcard' as const, region: 'usa' as const, species: 'Veemon' as const, name: 'NEW3', story: null, difficulty: 'auto' as const };
  const created = await backend.createBox(3, request);
  expect(created.pending_write).toBe(true);
  await backend.saveAs('/mock/copy.ps2', viewToEditSet(created.view), 'normal');
  expect((await backend.openSave('/mock/copy.ps2', undefined, 'ps2#1')).view).toEqual(first.view);
  expect((await backend.openSave('/mock/copy.ps2', undefined, 'ps2#3')).view.name).toBe('NEW3');
  await backend.createBox(4, { ...request, name: 'UNSAVED' });
  const reopened = await backend.openSave('/mock/copy.ps2', undefined, 'ps2#3');
  expect(reopened.empty_boxes).toContain(4);
  expect(reopened.pending_write).toBe(false);
});
