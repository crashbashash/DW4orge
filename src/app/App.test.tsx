import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { BackendProvider } from '../ipc/context';
import { createMockBackend } from '../ipc/mock';
import { App } from './App';

describe('App', () => {
  it('renders the shell under a backend', () => {
    render(
      <BackendProvider backend={createMockBackend()}>
        <App />
      </BackendProvider>,
    );
    expect(screen.getAllByText('DW4orge').length).toBeGreaterThan(0);
  });
});
