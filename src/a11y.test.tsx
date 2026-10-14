import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { App } from './app/App';
import { BackendProvider } from './ipc/context';
import { createMockBackend } from './ipc/mock';

const SECTIONS = ['Character', 'Items', 'Equipment', 'Disks', 'Story', 'Bank'];

/**
 * Whether an element carries some accessible name.
 *
 * A deliberate DOM-level check rather than the full `dom-accessibility-api`
 * algorithm: it is looking for a *missing* label, and Testing Library's own
 * `getByRole(..., { name })` queries (used throughout this suite) already
 * exercise the real name computation where a name matters.
 */
function hasAccessibleName(el: Element): boolean {
  for (const attr of ['aria-label', 'aria-labelledby', 'title']) {
    if (el.getAttribute(attr)?.trim()) return true;
  }
  if (el instanceof HTMLInputElement && el.labels && el.labels.length > 0) return true;
  if (el instanceof HTMLSelectElement && el.labels && el.labels.length > 0) return true;
  if (el instanceof HTMLTextAreaElement && el.labels && el.labels.length > 0) return true;
  // A button is named by its text; a wrapping <label> names a checkbox.
  return (el.textContent ?? '').trim().length > 0;
}

/** Interactive elements under `root` that carry no accessible name. */
function unlabelledControls(root: HTMLElement): string[] {
  const missing: string[] = [];
  for (const el of root.querySelectorAll('input, select, textarea, button')) {
    if (el.getAttribute('type') === 'hidden') continue;
    if (el.getAttribute('aria-hidden') === 'true' || el.closest('[aria-hidden="true"]')) continue;
    if (!hasAccessibleName(el)) {
      const id = el.className
        ? `${el.tagName.toLowerCase()}.${el.className}`
        : el.tagName.toLowerCase();
      missing.push(id);
    }
  }
  return missing;
}

async function loadSample() {
  render(
    <BackendProvider backend={createMockBackend()}>
      <App />
    </BackendProvider>,
  );
  await userEvent.click(await screen.findByRole('button', { name: /load sample save/i }));
  await screen.findAllByText('Dorumon');
}

describe('accessibility', () => {
  it('names every control in every section', async () => {
    await loadSample();
    const main = screen.getByRole('main');
    for (const section of SECTIONS) {
      await userEvent.click(screen.getByRole('button', { name: section }));
      expect(unlabelledControls(main), `unlabelled controls in ${section}`).toEqual([]);
    }
  });

  it('has a single h1 plus the landmarks', async () => {
    await loadSample();
    expect(screen.getAllByRole('heading', { level: 1 })).toHaveLength(1);
    expect(screen.getByRole('main')).toBeTruthy();
    expect(screen.getByRole('navigation', { name: /sections/i })).toBeTruthy();
  });

  it('opens the New Save dialog named and focused', async () => {
    await loadSample();
    await userEvent.click(screen.getByRole('button', { name: 'New' }));

    // `getByRole` computes the real accessible name, so this asserts the
    // dialog is labelled by its heading.
    const dialog = await screen.findByRole('dialog', { name: /new save/i });
    expect(within(dialog).getByRole('heading', { name: /new save/i })).toBeTruthy();
    expect(dialog.contains(document.activeElement)).toBe(true);
  });
});
