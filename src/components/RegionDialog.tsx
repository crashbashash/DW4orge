import type { Region } from '../bindings';
import { Modal } from './Modal';

export const REGION_LABELS: Record<Region, string> = {
  usa: 'USA', europe: 'Europe', japan: 'Japan (Digimon World X)',
};

/** A choice is required for mixed-region cards and raw-to-card exports. */
export function RegionDialog({ request, finish }: {
  request: { regions: Region[]; message: string } | null;
  finish: (region: Region | null) => void;
}) {
  return <Modal title="Choose save region" isOpen={request !== null} onClose={() => finish(null)}
    footer={<button type="button" onClick={() => finish(null)}>Cancel</button>}>
    <p>{request?.message}</p>
    <div className="grid">
      {request?.regions.map(region => <button type="button" key={region} onClick={() => finish(region)}>{REGION_LABELS[region]}</button>)}
    </div>
  </Modal>;
}
