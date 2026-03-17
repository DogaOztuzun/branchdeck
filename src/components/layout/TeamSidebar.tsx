import { createEffect, createMemo, createSignal, For, Show } from 'solid-js';
import { listAgentDefinitions } from '../../lib/commands/agent';
import { getAgentStore } from '../../lib/stores/agent';
import { getRepoStore } from '../../lib/stores/repo';
import { getTerminalStore } from '../../lib/stores/terminal';
import { statusColor } from '../../lib/utils';
import type { AgentDefinition } from '../../types/agent';
import { FileGrid } from './FileGrid';

export function TeamSidebar() {
  const repoStore = getRepoStore();
  const terminalStore = getTerminalStore();
  const agentStore = getAgentStore();
  const [definitions, setDefinitions] = createSignal<AgentDefinition[]>([]);

  const repoPath = () => repoStore.state.activeRepoPath ?? '';
  const worktreePath = () => repoStore.state.activeWorktreePath ?? '';

  createEffect(() => {
    const repo = repoPath();
    const wt = worktreePath();
    if (!repo) {
      setDefinitions([]);
      return;
    }
    // Scan both repo root and active worktree for agent definitions
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
        tab,
        agent: agentStore.getTabAgent(tab.id),
      }))
      .filter((a) => a.agent);
  });

  return (
    <div class="flex flex-col h-full bg-surface">
      {/* Header */}
      <div class="px-3 py-2 border-b border-border">
        <span class="text-xs font-bold uppercase text-text-muted tracking-wider">Team</span>
      </div>

      {/* File Grid */}
      <Show when={worktreePath()}>
        <div class="border-b border-border">
          <FileGrid worktreePath={worktreePath()} />
        </div>
      </Show>

      {/* Active Agents */}
      <Show when={activeAgents().length > 0}>
        <div class="px-2 py-1.5 border-b border-border">
          <span class="text-[10px] uppercase text-text-muted tracking-wider px-1">Active</span>
          <div class="mt-1 space-y-0.5">
            <For each={activeAgents()}>
              {(item) => (
                <div class="flex items-center gap-2 px-2 py-1 rounded text-xs hover:bg-bg/50">
                  <span
                    class={`w-2 h-2 rounded-full shrink-0 ${statusColor(item.agent?.status ?? 'stopped')}`}
                  />
                  <div class="flex-1 min-w-0">
                    <div class="text-text truncate">{item.tab.title}</div>
                    <Show when={item.agent?.currentTool}>
                      <div class="text-[10px] text-text-muted truncate">
                        {item.agent?.currentTool}
                        <Show when={item.agent?.currentFile}>
                          {' '}
                          <span class="opacity-60">{item.agent?.currentFile}</span>
                        </Show>
                      </div>
                    </Show>
                    <Show when={item.agent && item.agent.subagentCount > 0}>
                      <div class="text-[10px] text-info">
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
            <div class="px-3 py-4 text-xs text-text-muted text-center">
              <Show when={repoPath()} fallback={<span>Select a repo to see agents</span>}>
                <div>No agents defined</div>
                <div class="mt-1 text-[10px]">.claude/agents/*.md</div>
              </Show>
            </div>
          }
        >
          <div class="px-2 py-1.5">
            <span class="text-[10px] uppercase text-text-muted tracking-wider px-1">
              Definitions
            </span>
            <div class="mt-1 space-y-0.5">
              <For each={definitions()}>
                {(def) => (
                  <div class="group flex items-start gap-2 px-2 py-1.5 rounded hover:bg-bg/50">
                    <div class="flex-1 min-w-0">
                      <div class="text-xs text-text truncate">{def.name}</div>
                      <Show when={def.description}>
                        <div class="text-[10px] text-text-muted truncate">{def.description}</div>
                      </Show>
                      <div class="flex gap-1.5 mt-0.5">
                        <Show when={def.model}>
                          <span class="text-[9px] px-1 rounded bg-bg text-info">{def.model}</span>
                        </Show>
                        <Show when={def.permissionMode}>
                          <span class="text-[9px] px-1 rounded bg-bg text-warning">
                            {def.permissionMode}
                          </span>
                        </Show>
                      </div>
                    </div>
                    <button
                      type="button"
                      class="opacity-0 group-hover:opacity-100 shrink-0 mt-0.5 px-1.5 py-0.5 text-[10px] text-text-muted hover:text-text border border-border rounded hover:border-primary cursor-pointer transition-opacity"
                      onClick={() => launchAgent(def)}
                      disabled={!worktreePath()}
                    >
                      Launch
                    </button>
                  </div>
                )}
              </For>
            </div>
          </div>
        </Show>
      </div>

      {/* Footer */}
      <Show when={worktreePath()}>
        <div class="p-2 border-t border-border">
          <button
            type="button"
            class="w-full px-3 py-1.5 text-xs text-text-muted hover:text-text cursor-pointer text-left hover:bg-bg/50 rounded"
            onClick={() => terminalStore.openClaudeTab(worktreePath())}
          >
            + New Claude Session
          </button>
        </div>
      </Show>
    </div>
  );
}
