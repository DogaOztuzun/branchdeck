import { createEffect, createMemo, createSignal, For, Show } from 'solid-js';
import { listAgentDefinitions } from '../../lib/commands/agent';
import { getAgentStore } from '../../lib/stores/agent';
import { getRepoStore } from '../../lib/stores/repo';
import { getTerminalStore } from '../../lib/stores/terminal';
import { statusColor } from '../../lib/utils';
import type { AgentDefinition } from '../../types/agent';
import { Button } from '../ui/Button';
import { SectionHeader } from '../ui/SectionHeader';

export function AgentsPanel() {
  const repoStore = getRepoStore();
  const terminalStore = getTerminalStore();
  const agentStore = getAgentStore();
  const [definitions, setDefinitions] = createSignal<AgentDefinition[]>([]);
  const [agentsCollapsed, setAgentsCollapsed] = createSignal(false);

  const repoPath = () => repoStore.state.activeRepoPath ?? '';
  const worktreePath = () => repoStore.state.activeWorktreePath ?? '';

  createEffect(() => {
    const repo = repoPath();
    const wt = worktreePath();
    if (!repo) {
      setDefinitions([]);
      return;
    }
    const paths = [repo];
    if (wt && wt !== repo) paths.push(wt);
    Promise.all(paths.map((p) => listAgentDefinitions(p).catch(() => [])))
      .then((results) => {
        const seen = new Set<string>();
        const merged: AgentDefinition[] = [];
        for (const defs of results) {
          for (const def of defs) {
            if (!seen.has(def.name)) {
              seen.add(def.name);
              merged.push(def);
            }
          }
        }
        setDefinitions(merged);
      })
      .catch(() => setDefinitions([]));
  });

  function launchAgent(def: AgentDefinition) {
    const wt = worktreePath();
    if (!wt) return;
    terminalStore.openAgentTab(wt, def.name);
  }

  const activeAgents = createMemo(() => {
    const visible = terminalStore.state.tabs.filter(
      (t) => t.type === 'claude' && t.worktreePath === worktreePath(),
    );
    return visible
      .map((tab) => ({
        tab: { id: tab.id, title: tab.title },
        agent: agentStore.getTabAgent(tab.id),
      }))
      .filter((a) => a.agent);
  });

  return (
    <div class="flex flex-col h-full bg-bg-sidebar">
      {/* Header */}
      <div class="px-3 py-2 border-b border-border-subtle">
        <span class="text-[10px] font-bold uppercase text-text-dim tracking-wider">Agents</span>
      </div>

      {/* Active Agents */}
      <Show when={activeAgents().length > 0}>
        <div class="border-b border-border-subtle">
          <SectionHeader label="Active" count={activeAgents().length} />
          <div class="mt-1 space-y-0.5">
            <For each={activeAgents()}>
              {(item) => (
                <div class="flex items-center gap-2 px-3 py-1 text-xs hover:bg-bg-main/30">
                  <span
                    class={`w-2 h-2 rounded-full shrink-0 ${statusColor(item.agent?.status ?? 'stopped')}`}
                  />
                  <div class="flex-1 min-w-0">
                    <div class="text-text-main truncate">{item.tab.title}</div>
                    <Show when={item.agent?.currentTool}>
                      <div class="text-[10px] text-text-dim truncate">
                        {item.agent?.currentTool}
                        <Show when={item.agent?.currentFile}>
                          {' '}
                          <span class="opacity-60">{item.agent?.currentFile}</span>
                        </Show>
                      </div>
                    </Show>
                    <Show when={item.agent && item.agent.subagentCount > 0}>
                      <div class="text-[10px] text-accent-info">
                        +{item.agent?.subagentCount} subagent
                        {item.agent?.subagentCount === 1 ? '' : 's'}
                      </div>
                    </Show>
                  </div>
                </div>
              )}
            </For>
          </div>
        </div>
      </Show>

      {/* Agent Definitions */}
      <div class="flex-1 overflow-y-auto">
        <Show
          when={definitions().length > 0}
          fallback={
            <div class="px-3 py-6 text-xs text-text-dim text-center">
              <Show when={repoPath()} fallback={<span>Select a repository to see agents</span>}>
                <div>No Claude agents defined</div>
                <div class="mt-2 text-[10px]">Open a Claude Code terminal to get started</div>
                <div class="mt-1 text-[10px] text-text-dim/50">
                  or add agents in .claude/agents/*.md
                </div>
              </Show>
            </div>
          }
        >
          <div>
            <SectionHeader
              label="Definitions"
              count={definitions().length}
              collapsed={agentsCollapsed()}
              onToggle={() => setAgentsCollapsed((v) => !v)}
            />
            <Show when={!agentsCollapsed()}>
              <div class="pb-1 space-y-0.5">
                <For each={definitions()}>
                  {(def) => (
                    <div class="group flex items-start gap-2 px-3 py-1.5 hover:bg-bg-main/30">
                      <div class="flex-1 min-w-0">
                        <div class="text-xs text-text-main truncate">{def.name}</div>
                        <Show when={def.description}>
                          <div class="text-[10px] text-text-dim truncate">{def.description}</div>
                        </Show>
                        <div class="flex gap-1.5 mt-0.5">
                          <Show when={def.model}>
                            <span class="text-[9px] px-1 bg-bg-main text-accent-info">
                              {def.model}
                            </span>
                          </Show>
                          <Show when={def.permissionMode}>
                            <span class="text-[9px] px-1 bg-bg-main text-accent-warning">
                              {def.permissionMode}
                            </span>
                          </Show>
                        </div>
                      </div>
                      <button
                        type="button"
                        class="opacity-0 group-hover:opacity-100 shrink-0 mt-0.5 px-1.5 py-0.5 text-[10px] text-text-dim hover:text-text-main border border-border-subtle hover:border-accent-primary cursor-pointer transition-opacity"
                        onClick={() => launchAgent(def)}
                        disabled={!worktreePath()}
                      >
                        Launch
                      </button>
                    </div>
                  )}
                </For>
              </div>
            </Show>
          </div>
        </Show>
      </div>

      {/* Footer */}
      <Show when={worktreePath()}>
        <div class="p-2 border-t border-border-subtle">
          <Button
            variant="ghost"
            size="compact"
            class="w-full justify-start"
            onClick={() => terminalStore.openClaudeTab(worktreePath())}
          >
            + New Claude Session
          </Button>
        </div>
      </Show>
    </div>
  );
}
