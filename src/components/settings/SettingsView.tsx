import { getSATStore } from '../../lib/stores/sat';
import { SATSettings } from './SATSettings';
import { SaveBar } from './SaveBar';
import { SettingsPreview } from './SettingsPreview';

export function SettingsView() {
  const sat = getSATStore();

  const totalFindings = () => sat.findings().length;
  const falsePositives = () => sat.findings().filter((f) => f.status === 'false-positive').length;

  return (
    <div class="flex-1 flex flex-col overflow-hidden">
      <div class="flex-1 overflow-y-auto">
        <div class="mx-auto max-w-[600px] pt-4 pb-16 px-3">
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
