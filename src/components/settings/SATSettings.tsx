import { getSettingsStore } from '../../lib/stores/settings';

const severityOptions = [
  { label: 'Critical only', value: 'critical' as const },
  { label: 'High + Critical', value: 'high' as const },
  { label: 'Medium+', value: 'medium' as const },
  { label: 'All', value: 'all' as const },
];

const scheduleOptions = [
  { label: 'Manual', value: 'manual' as const },
  { label: 'Nightly', value: 'nightly' as const },
  { label: 'Post-merge', value: 'post-merge' as const },
];

export function SATSettings() {
  const settings = getSettingsStore();
  const d = () => settings.draft();

  return (
    <div class="space-y-4">
      {/* Severity threshold */}
      <div>
        <label for="sat-severity" class="text-sm font-medium text-text-main block mb-1">
          Minimum severity for auto-issue
        </label>
        <select
          id="sat-severity"
          class="h-8 w-full max-w-xs bg-[#14141b] border border-border-subtle px-2 text-base text-text-main outline-none focus:border-accent-primary"
          value={d().minSeverity}
          onChange={(e) =>
            settings.updateDraft(
              'minSeverity',
              e.currentTarget.value as 'critical' | 'high' | 'medium' | 'all',
            )
          }
        >
          {severityOptions.map((o) => (
            <option value={o.value}>{o.label}</option>
          ))}
        </select>
        <p class="text-[11px] text-text-dim mt-1">
          Only findings at or above this severity will create GitHub issues
        </p>
      </div>

      {/* Category filters */}
      <div>
        <span class="text-sm font-medium text-text-main block mb-1">Category filters</span>
        <div class="space-y-1.5">
          <label class="flex items-center gap-2 text-base text-text-main cursor-pointer">
            <input
              type="checkbox"
              checked={d().categoryFilters.app}
              disabled
              class="accent-accent-primary"
            />
            App issues
            <span class="text-[11px] text-text-dim">Always enabled</span>
          </label>
          <label class="flex items-center gap-2 text-base text-text-main cursor-pointer">
            <input
              type="checkbox"
              checked={d().categoryFilters.runner}
              onChange={(e) => settings.updateCategoryFilter('runner', e.currentTarget.checked)}
              class="accent-accent-primary"
            />
            Runner issues
            <span class="text-[11px] text-text-dim">WebDriver artifacts</span>
          </label>
          <label class="flex items-center gap-2 text-base text-text-main cursor-pointer">
            <input
              type="checkbox"
              checked={d().categoryFilters.scenario}
              onChange={(e) => settings.updateCategoryFilter('scenario', e.currentTarget.checked)}
              class="accent-accent-primary"
            />
            Scenario issues
            <span class="text-[11px] text-text-dim">Test quality problems</span>
          </label>
        </div>
      </div>

      {/* Confidence threshold */}
      <div>
        <label for="sat-confidence" class="text-sm font-medium text-text-main block mb-1">
          Confidence threshold
        </label>
        <input
          id="sat-confidence"
          type="number"
          min="0"
          max="100"
          class="h-8 w-24 bg-[#14141b] border border-border-subtle px-2 text-base text-text-main outline-none focus:border-accent-primary"
          value={d().confidenceThreshold}
          onInput={(e) =>
            settings.updateDraft('confidenceThreshold', Number(e.currentTarget.value))
          }
        />
        <p class="text-[11px] text-text-dim mt-1">
          Findings below this score are logged but don't create issues
        </p>
      </div>

      {/* Docs path */}
      <div>
        <label for="sat-docs" class="text-sm font-medium text-text-main block mb-1">
          Project docs path
        </label>
        <input
          id="sat-docs"
          type="text"
          class="h-8 w-full max-w-md bg-[#14141b] border border-border-subtle px-2 text-base text-text-main outline-none focus:border-accent-primary"
          value={d().docsPath}
          onInput={(e) => settings.updateDraft('docsPath', e.currentTarget.value)}
          placeholder="docs/"
        />
        <p class="text-[11px] text-text-dim mt-1">Directory used for SAT scenario generation</p>
      </div>

      {/* Run schedule */}
      <div>
        <label for="sat-schedule" class="text-sm font-medium text-text-main block mb-1">
          Run schedule
        </label>
        <select
          id="sat-schedule"
          class="h-8 w-full max-w-xs bg-[#14141b] border border-border-subtle px-2 text-base text-text-main outline-none focus:border-accent-primary"
          value={d().runSchedule}
          onChange={(e) =>
            settings.updateDraft(
              'runSchedule',
              e.currentTarget.value as 'manual' | 'nightly' | 'post-merge',
            )
          }
        >
          {scheduleOptions.map((o) => (
            <option value={o.value}>{o.label}</option>
          ))}
        </select>
      </div>
    </div>
  );
}
