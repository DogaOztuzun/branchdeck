import { getLayoutStore } from '../../lib/stores/layout';
import { getRepoStore } from '../../lib/stores/repo';
import { getSATStore } from '../../lib/stores/sat';
import { getSetupStore } from '../../lib/stores/setup';
import { Button } from '../ui/Button';
import { SATSettings } from './SATSettings';
import { SaveBar } from './SaveBar';
import { SettingsPreview } from './SettingsPreview';

export function SettingsView() {
  const sat = getSATStore();
  const setup = getSetupStore();
  const layout = getLayoutStore();
  const repo = getRepoStore();

  const totalFindings = () => sat.findings().length;
  const falsePositives = () => sat.findings().filter((f) => f.status === 'false-positive').length;

  function rerunSetup() {
    const activeRepo = repo.getActiveRepo();
    if (activeRepo) {
      setup.startSetup(activeRepo.path);
    }
    layout.setActiveView('setup');
  }

  return (
    <div class="flex-1 flex flex-col overflow-hidden">
      <div class="flex-1 overflow-y-auto">
        <div class="mx-auto max-w-[600px] pt-4 pb-16 px-3">
          <div class="mb-6 py-3 px-3 bg-bg-sidebar border border-border-subtle">
            <div class="flex items-center justify-between">
              <div>
                <h2 class="text-sm font-semibold text-text-main">Project Setup</h2>
                <p class="text-xs text-text-dim mt-0.5">
                  Re-run the guided setup to modify project configuration.
                </p>
              </div>
              <Button variant="secondary" size="compact" onClick={rerunSetup}>
                Re-run Setup
              </Button>
            </div>
          </div>
          <h1 class="text-lg font-semibold text-text-main mb-4">SAT Configuration</h1>
          <SATSettings />
          <div class="mt-4">
            <SettingsPreview totalFindings={totalFindings()} falsePositives={falsePositives()} />
          </div>
        </div>
      </div>
      <SaveBar />
    </div>
  );
}
