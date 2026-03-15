import { createStore, produce } from 'solid-js/store';
import type { FileStatus, RepoInfo, WorktreeInfo } from '../../types/git';
import {
  addRepository,
  createWorktree as createWorktreeCmd,
  getRepoStatus,
  listRepositories,
  listWorktrees,
  removeRepository,
} from '../commands/git';
import { getAppConfig, getRepoConfig, saveAppConfig, saveRepoConfig } from '../commands/workspace';

type RepoState = {
  repos: RepoInfo[];
  activeRepo: RepoInfo | null;
  worktreesByRepo: Record<string, WorktreeInfo[]>;
  activeWorktree: WorktreeInfo | null;
  statuses: FileStatus[];
};

function createRepoStore() {
  const [state, setState] = createStore<RepoState>({
    repos: [],
    activeRepo: null,
    worktreesByRepo: {},
    activeWorktree: null,
    statuses: [],
  });

  async function loadRepos() {
    const repos = await listRepositories();
    setState('repos', repos);
  }

  async function restoreLastSession() {
    await loadRepos();

    const config = await getAppConfig();
    if (!config.lastActiveRepo || state.repos.length === 0) return;

    const lastRepo = state.repos.find((r) => r.path === config.lastActiveRepo);
    if (!lastRepo) return;

    setState('activeRepo', lastRepo);
    const wts = await listWorktrees(lastRepo.path);
    setState('worktreesByRepo', lastRepo.path, wts);

    const repoConfig = await getRepoConfig(lastRepo.path);
    const targetWt = repoConfig.lastWorktree
      ? wts.find((w) => w.path === repoConfig.lastWorktree)
      : wts.find((w) => w.isMain);

    if (targetWt) {
      await selectWorktree(targetWt);
    }
  }

  async function persistState() {
    try {
      const config = await getAppConfig();
      config.lastActiveRepo = state.activeRepo?.path ?? null;
      await saveAppConfig(config);

      if (state.activeRepo && state.activeWorktree) {
        const repoConfig = await getRepoConfig(state.activeRepo.path);
        repoConfig.lastWorktree = state.activeWorktree.path;
        await saveRepoConfig(state.activeRepo.path, repoConfig);
      }
    } catch {
      // Config save is best-effort
    }
  }

  async function addRepo() {
    const repo = await addRepository();
    if (repo) {
      setState(
        produce((s) => {
          s.repos.push(repo);
        }),
      );
      await selectRepo(repo);
    }
  }

  async function removeRepo(repoPath: string) {
    await removeRepository(repoPath);
    setState(
      produce((s) => {
        s.repos = s.repos.filter((r) => r.path !== repoPath);
        delete s.worktreesByRepo[repoPath];
        if (s.activeRepo?.path === repoPath) {
          s.activeRepo = null;
          s.activeWorktree = null;
          s.statuses = [];
        }
      }),
    );
    persistState();
  }

  async function selectRepo(repo: RepoInfo) {
    setState('activeRepo', repo);
    const wts = await listWorktrees(repo.path);
    setState('worktreesByRepo', repo.path, wts);
    const main = wts.find((w) => w.isMain);
    if (main) {
      await selectWorktree(main);
    } else {
      setState('activeWorktree', null);
      setState('statuses', []);
      persistState();
    }
  }

  async function selectWorktree(worktree: WorktreeInfo) {
    setState('activeWorktree', worktree);
    await refreshStatus();
    persistState();
  }

  async function createWorktree(repoPath: string, name: string, branch?: string) {
    const wt = await createWorktreeCmd(repoPath, name, branch);
    setState(
      produce((s) => {
        const existing = s.worktreesByRepo[repoPath] ?? [];
        existing.push(wt);
        s.worktreesByRepo[repoPath] = existing;
      }),
    );
    return wt;
  }

  async function refreshStatus() {
    if (!state.activeWorktree) {
      setState('statuses', []);
      return;
    }
    const statuses = await getRepoStatus(state.activeWorktree.path);
    setState('statuses', statuses);
  }

  function getWorktrees(repoPath: string): WorktreeInfo[] {
    return state.worktreesByRepo[repoPath] ?? [];
  }

  return {
    state,
    loadRepos,
    restoreLastSession,
    addRepo,
    removeRepo,
    selectRepo,
    selectWorktree,
    createWorktree,
    refreshStatus,
    getWorktrees,
  };
}

let store: ReturnType<typeof createRepoStore> | undefined;

export function getRepoStore() {
  if (!store) store = createRepoStore();
  return store;
}
