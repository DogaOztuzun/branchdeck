import { createEffect, createMemo, createSignal, For, onCleanup, Show } from 'solid-js';
import { listAgentDefinitions } from '../../lib/commands/agent';
import {
  cancelRun,
  launchRun,
  respondToPermission,
  resumeRun,
  retryRun,
} from '../../lib/commands/run';
import { getAgentStore } from '../../lib/stores/agent';
import { getRepoStore } from '../../lib/stores/repo';
import { getTaskStore } from '../../lib/stores/task';
import { getTerminalStore } from '../../lib/stores/terminal';
import { parseArtifactSummary, statusColor } from '../../lib/utils';
import type { AgentDefinition } from '../../types/agent';
import type { TaskInfo } from '../../types/task';
import { ApprovalDialog } from '../task/ApprovalDialog';
import { CreateTaskModal } from '../task/CreateTaskModal';
import { RunTimeline } from '../task/RunTimeline';
import { TaskBadge } from '../task/TaskBadge';
import { SectionHeader } from '../ui/SectionHeader';
import { FileGrid } from './FileGrid';

export function TeamSidebar() {
  const repoStore = getRepoStore();
  const terminalStore = getTerminalStore();
  const agentStore = getAgentStore();
  const taskStore = getTaskStore();
  const [definitions, setDefinitions] = createSignal<AgentDefinition[]>([]);
  const [showCreateTask, setShowCreateTask] = createSignal(false);
  const [launchError, setLaunchError] = createSignal<string | null>(null);
  const [prContextCollapsed, setPrContextCollapsed] = createSignal(false);
  const [knowledgeCollapsed, setKnowledgeCollapsed] = createSignal(false);
  const [agentsCollapsed, setAgentsCollapsed] = createSignal(false);
  let launchErrorTimer: ReturnType<typeof setTimeout> | undefined;

  const repoPath = () => repoStore.state.activeRepoPath ?? '';
  const worktreePath = () => repoStore.state.activeWorktreePath ?? '';

  // Reset modal state when worktree changes
  createEffect(() => {
    worktreePath(); // track dependency
    setShowCreateTask(false);
  });

  onCleanup(() => clearTimeout(launchErrorTimer));

  const tasksWithWorktree = createMemo(() => {
    const tasks = taskStore.state.tasksByWorktree;
    const activeRepo = repoStore.state.activeRepoPath;
    if (!activeRepo) return [];
    const wts = repoStore.state.worktreesByRepo[activeRepo] ?? [];
    const result: { worktree: (typeof wts)[number]; task: TaskInfo }[] = [];
    for (const wt of wts) {
      // Normalize path to match store keys (ensure trailing slash)
      const key = wt.path.endsWith('/') ? wt.path : `${wt.path}/`;
      const task = tasks[key];
      if (task) {
        result.push({ worktree: wt, task });
      }
    }
    return result;
  });

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

  const activeWorktree = createMemo(() => {
    const activeRepo = repoStore.state.activeRepoPath;
    const activeWt = repoStore.state.activeWorktreePath;
    if (!activeRepo || !activeWt) return null;
    const wts = repoStore.state.worktreesByRepo[activeRepo] ?? [];
    return wts.find((wt) => wt.path === activeWt) ?? null;
  });

  const activeWorktreeHasTask = createMemo(() => {
    const wt = worktreePath();
    if (!wt) return false;
    return taskStore.hasTaskForWorktree(wt);
  });

  const canLaunch = createMemo(() => !taskStore.state.activeRun);

  function handleLaunch(task: TaskInfo, wtPath: string) {
    if (!canLaunch()) return;
    clearTimeout(launchErrorTimer);
    setLaunchError(null);
    launchRun(task.path, wtPath).catch((e) => {
      setLaunchError(String(e));
      launchErrorTimer = setTimeout(() => setLaunchError(null), 5000);
    });
  }

  function launchAgent(def: AgentDefinition) {
    const wt = worktreePath();
    if (!wt) return;
    terminalStore.openAgentTab(wt, def.name);
  }

  const activeAgents = createMemo(() => {
    const visible = terminalStore.state.tabs.filter(
      (t) => t.type === 'claude' && t.worktreePath === worktreePath(),
    );
    const terminalAgents = visible
      .map((tab) => ({
        tab: { id: tab.id, title: tab.title },
        agent: agentStore.getTabAgent(tab.id),
      }))
      .filter((a) => a.agent);

    // Include task agent if running
    const run = taskStore.state.activeRun;
    if (
      run?.tabId &&
      (run.status === 'running' || run.status === 'starting' || run.status === 'blocked')
    ) {
      const taskAgent = agentStore.getTabAgent(run.tabId);
      if (taskAgent) {
        terminalAgents.push({
          tab: { id: run.tabId, title: 'Task Agent' },
          agent: taskAgent,
        });
      }
    }

    return terminalAgents;
  });

  return (
    <div class="flex flex-col h-full bg-bg-sidebar">
      {/* Header */}
      <div class="px-3 py-2 border-b border-border-subtle">
        <span class="text-xs font-bold uppercase text-text-dim tracking-wider">Team</span>
      </div>

      {/* File Grid */}
      <Show when={worktreePath()}>
        <div class="border-b border-border-subtle">
          <FileGrid worktreePath={worktreePath()} />
        </div>
      </Show>

      {/* Tasks */}
      <div class="px-2 py-1.5 border-b border-border-subtle">
        <div class="flex items-center justify-between px-1">
          <span class="text-[10px] uppercase text-text-dim tracking-wider">Tasks</span>
          <Show when={worktreePath() && !activeWorktreeHasTask()}>
            <button
              type="button"
              class="text-[10px] text-text-dim hover:text-text-main cursor-pointer"
              onClick={() => setShowCreateTask(true)}
            >
              + New Task
            </button>
          </Show>
        </div>
        <Show
          when={tasksWithWorktree().length > 0}
          fallback={
            <Show when={worktreePath()}>
              <div class="px-2 py-2 text-[10px] text-text-dim text-center">No tasks yet</div>
            </Show>
          }
        >
          <div class="mt-1 divide-y divide-border-subtle/20">
            <For each={tasksWithWorktree()}>
              {(item) => (
                <div class="px-3 py-2 text-xs hover:bg-bg-main/30 transition-colors duration-150">
                  {/* Row 1: branch name (full width) */}
                  <div class="text-[11px] text-text-main font-medium truncate">{item.worktree.branch}</div>
                  {/* Row 2: type + badge */}
                  <div class="flex items-center justify-between mt-0.5">
                    <span class="text-[10px] text-text-dim capitalize">{item.task.frontmatter.type.replace('-', ' ')}</span>
                    <TaskBadge status={item.task.frontmatter.status} />
                  </div>
                  {/* Row 3: metadata + actions */}
                  <div class="flex items-center gap-2 mt-1 text-[10px] text-text-dim">
                    <Show when={item.task.frontmatter.pr}>
                      <span>PR #{item.task.frontmatter.pr}</span>
                    </Show>
                    <Show when={item.task.frontmatter['run-count'] > 0}>
                      <span>{item.task.frontmatter['run-count']} runs</span>
                    </Show>
                    <Show when={parseArtifactSummary(item.task.body)}>
                      {(artifacts) => (
                        <>
                          <Show when={artifacts().totalCommits > 0}>
                            <span>{artifacts().totalCommits} commit{artifacts().totalCommits === 1 ? '' : 's'}</span>
                          </Show>
                          <Show when={artifacts().pr}>
                            <span class="text-accent-info">PR #{artifacts().pr}</span>
                          </Show>
                        </>
                      )}
                    </Show>
                    {/* Action buttons */}
                    <div class="ml-auto flex items-center gap-1">
                      <Show when={canLaunch() && item.task.frontmatter.status === 'created'}>
                        <button
                          type="button"
                          class="px-1.5 py-0.5 text-[10px] text-text-dim hover:text-text-main border border-border-subtle hover:border-accent-primary cursor-pointer"
                          onClick={() => handleLaunch(item.task, item.worktree.path)}
                        >
                          Launch
                        </button>
                      </Show>
                      <Show when={taskStore.state.activeRun && item.task.frontmatter.status === 'running'}>
                        <button
                          type="button"
                          class="px-1.5 py-0.5 text-[10px] text-red-400 hover:text-red-300 border border-border-subtle hover:border-red-400 cursor-pointer"
                          onClick={() => cancelRun().catch(() => {})}
                        >
                          Cancel
                        </button>
                      </Show>
                      <Show when={canLaunch() && (item.task.frontmatter.status === 'failed' || item.task.frontmatter.status === 'cancelled')}>
                        <button
                          type="button"
                          class="px-1.5 py-0.5 text-[10px] text-text-dim hover:text-text-main border border-border-subtle hover:border-accent-primary cursor-pointer"
                          onClick={() => retryRun(item.task.path, item.worktree.path).catch(() => {})}
                        >
                          Retry
                        </button>
                        <button
                          type="button"
                          class="px-1.5 py-0.5 text-[10px] text-text-dim hover:text-text-main border border-border-subtle hover:border-accent-info cursor-pointer"
                          onClick={() => resumeRun(item.task.path, item.worktree.path).catch(() => {})}
                        >
                          Resume
                        </button>
                      </Show>
                    </div>
                  </div>
                </div>
              )}
            </For>
          </div>
        </Show>
        <Show when={launchError()}>
          <p class="px-2 mt-1 text-[10px] text-accent-error truncate">{launchError()}</p>
        </Show>
      </div>
      {/* PR Context + Prior Knowledge for pr-shepherd tasks */}
      <For each={tasksWithWorktree()}>
        {(item) => (
          <Show when={item.task.frontmatter.type === 'pr-shepherd'}>
            <Show when={item.task.frontmatter.pr}>
              <div class="border-b border-border-subtle">
                <SectionHeader
                  label={`PR #${item.task.frontmatter.pr}`}
                  collapsed={prContextCollapsed()}
                  onToggle={() => setPrContextCollapsed((v) => !v)}
                />
                <Show when={!prContextCollapsed()}>
                  <div class="px-3 pb-2 text-[10px] space-y-1.5">
                    {/* Checks */}
                    <Show when={item.task.body.includes('Failing checks:')}>
                      <div class="flex items-center gap-2">
                        <span class="text-text-dim w-14 shrink-0">Checks</span>
                        <span class="text-accent-error">
                          {item.task.body
                            .split('\n')
                            .find((l) => l.includes('Failing checks:'))
                            ?.replace('- Failing checks: ', '')
                            ?.replace('- ', '') ?? 'failing'}
                        </span>
                      </div>
                    </Show>
                    <Show when={!item.task.body.includes('Failing checks:')}>
                      <div class="flex items-center gap-2">
                        <span class="text-text-dim w-14 shrink-0">Checks</span>
                        <span class="text-accent-success">passing</span>
                      </div>
                    </Show>
                    {/* Reviews */}
                    <div class="flex items-center gap-2">
                      <span class="text-text-dim w-14 shrink-0">Reviews</span>
                      <span class="text-text-main">
                        {item.task.body
                          .split('\n')
                          .find((l) => l.includes('Reviews:'))
                          ?.replace('- Reviews: ', '')
                          ?.replace('- ', '') ?? 'None'}
                      </span>
                    </div>
                    {/* Diff */}
                    <Show when={item.task.body.includes('Diff:')}>
                      <div class="flex items-center gap-2">
                        <span class="text-text-dim w-14 shrink-0">Diff</span>
                        {(() => {
                          const line = item.task.body
                            .split('\n')
                            .find((l) => l.includes('Diff:'))
                            ?.replace('- Diff: ', '')
                            ?.replace('- ', '') ?? '';
                          const addMatch = line.match(/\+(\d+)/);
                          const delMatch = line.match(/-(\d+)/);
                          const fileMatch = line.match(/(\d+)\s*(?:file|across)/);
                          return (
                            <span>
                              <Show when={addMatch}>
                                <span class="text-green-400">+{addMatch?.[1]}</span>
                              </Show>
                              {' '}
                              <Show when={delMatch}>
                                <span class="text-red-400">-{delMatch?.[1]}</span>
                              </Show>
                              <Show when={fileMatch}>
                                <span class="text-text-dim"> {fileMatch?.[1]} files</span>
                              </Show>
                            </span>
                          );
                        })()}
                      </div>
                    </Show>
                  </div>
                </Show>
              </div>
            </Show>
            <Show
              when={
                item.task.body.includes('## Prior Knowledge') &&
                !item.task.body.includes('(none yet)')
              }
            >
              <div class="border-b border-border-subtle">
                <SectionHeader
                  label="Prior Knowledge"
                  collapsed={knowledgeCollapsed()}
                  onToggle={() => setKnowledgeCollapsed((v) => !v)}
                  count={
                    item.task.body
                      .split('## Prior Knowledge')[1]
                      ?.split('\n')
                      .filter((l) => l.startsWith('- ')).length
                  }
                />
                <Show when={!knowledgeCollapsed()}>
                  <div class="px-3 pb-2">
                    <div class="text-[10px] text-text-dim space-y-0.5 bg-bg-main/30 px-2 py-1.5 border border-border-subtle/30 max-h-32 overflow-y-auto">
                      <For
                        each={
                          item.task.body
                            .split('## Prior Knowledge')[1]
                            ?.split('\n')
                            .filter((l) => l.startsWith('- ')) ?? []
                        }
                      >
                        {(line) => (
                          <div class="text-[10px] leading-relaxed break-words">{line.replace('- ', '')}</div>
                        )}
                      </For>
                    </div>
                  </div>
                </Show>
              </div>
            </Show>
          </Show>
        )}
      </For>

      <Show when={taskStore.state.runLog.length > 0 || taskStore.state.activeRun}>
        <RunTimeline
          entries={taskStore.state.runLog}
          visible={true}
          activeRun={taskStore.state.activeRun}
        />
      </Show>

      {/* Permission Approval Dialogs */}
      <For each={taskStore.state.pendingPermissions}>
        {(perm) => (
          <div class="border-b border-border-subtle">
            <ApprovalDialog
              permission={perm}
              onRespond={(decision) => {
                taskStore.removePermission(perm.toolUseId);
                respondToPermission(perm.toolUseId, decision).catch(() => {});
              }}
            />
          </div>
        )}
      </For>

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
            <div class="px-3 py-4 text-xs text-text-dim text-center">
              <Show when={repoPath()} fallback={<span>Select a repo to see agents</span>}>
                <div>No agents defined</div>
                <div class="mt-1 text-[10px]">.claude/agents/*.md</div>
              </Show>
            </div>
          }
        >
          <div>
            <SectionHeader
              label="Agents"
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
            </div>
          </div>
        </Show>
      </div>

      {/* Footer */}
      <Show when={worktreePath()}>
        <div class="p-2 border-t border-border-subtle">
          <button
            type="button"
            class="w-full px-3 py-1.5 text-xs text-text-dim hover:text-text-main cursor-pointer text-left hover:bg-bg-main/50"
            onClick={() => terminalStore.openClaudeTab(worktreePath())}
          >
            + New Claude Session
          </button>
        </div>
      </Show>

      {/* Create Task Modal */}
      <Show when={activeWorktree()}>
        {(wt) => (
          <CreateTaskModal
            open={showCreateTask()}
            worktreePath={wt().path}
            repo={repoStore.getActiveRepo()?.name ?? ''}
            branch={wt().branch}
            onClose={() => setShowCreateTask(false)}
          />
        )}
      </Show>
    </div>
  );
}
