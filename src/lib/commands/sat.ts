import type { SatPipelineResult } from '../../types/sat';
import { apiPost } from '../api/client';

/**
 * Trigger a complete SAT quality audit cycle.
 *
 * Chains: generate -> execute -> score -> create issues.
 * Returns the pipeline result with per-stage status and timing.
 */
export async function triggerSatCycle(projectRoot: string): Promise<SatPipelineResult> {
  try {
    return await apiPost<SatPipelineResult>('/sat/cycle', { projectRoot });
  } catch (e) {
    console.error(`triggerSatCycle failed: ${e}`);
    throw e;
  }
}
