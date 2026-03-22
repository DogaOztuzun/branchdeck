import { Show } from 'solid-js';
import type { TabAgentInfo } from '../../lib/stores/agent';
import { shortPath, statusColor } from '../../lib/utils';

type AgentBadgeProps = {
  agent: TabAgentInfo | undefined;
};

export function AgentBadge(props: AgentBadgeProps) {
  return (
    <Show when={props.agent}>
      {(agent) => (
        <span class="inline-flex items-center gap-1 ml-1.5">
          <span class={`w-1.5 h-1.5 rounded-full ${statusColor(agent().status)}`} />
          <Show when={agent().currentTool}>
            <span class="text-text-dim text-xs max-w-24 truncate">
              {agent().currentTool}
              <Show when={agent().currentFile}>
                {(file) => <span class="opacity-60"> {shortPath(file())}</span>}
              </Show>
            </span>
          </Show>
          <Show when={agent().subagentCount > 0}>
            <span class="text-accent-info text-xs">+{agent().subagentCount}</span>
          </Show>
        </span>
      )}
    </Show>
  );
}
