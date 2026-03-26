import { invoke } from '@tauri-apps/api/core';
import { error as logError } from '@tauri-apps/plugin-log';
import type { SatPipelineResult } from '../../types/sat';

/**
 * Trigger a complete SAT quality audit cycle.
 *
 * Chains: generate -> execute -> score -> create issues.
 * Returns the pipeline result with per-stage status and timing.
 */
export async function triggerSatCycle(projectRoot: string): Promise<SatPipelineResult> {
  try {
    return await invoke<SatPipelineResult>('trigger_sat_cycle', { projectRoot });
  } catch (e) {
    logError(`triggerSatCycle failed: ${e}`);
    throw e;
  }
}
