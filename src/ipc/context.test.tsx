import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { BackendProvider, useBackend } from './context';
import { createMockBackend } from './mock';

function Probe() {
  const backend = useBackend();
  return <span>{backend ? 'has backend' : 'missing'}</span>;
}

describe('useBackend', () => {
  it('provides the backend to descendants', () => {
    render(
      <BackendProvider backend={createMockBackend()}>
        <Probe />
      </BackendProvider>,
    );
    expect(screen.getByText('has backend')).toBeTruthy();
  });

  it('throws outside a provider', () => {
    expect(() => render(<Probe />)).toThrow(/BackendProvider/);
  });
});
