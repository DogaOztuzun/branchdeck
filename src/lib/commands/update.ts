import { invoke } from '@tauri-apps/api/core';
import { error as logError } from '@tauri-apps/plugin-log';

export async function installUpdate(): Promise<void> {
  try {
    await invoke('install_update');
  } catch (e) {
    logError(`installUpdate failed: ${e}`);
    throw e;
  }
}
