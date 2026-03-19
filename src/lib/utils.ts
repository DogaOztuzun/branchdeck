export function statusColor(status: string): string {
  switch (status) {
    case 'active':
      return 'bg-success';
    case 'idle':
      return 'bg-warning';
    case 'stopped':
      return 'bg-text-muted';
    default:
      return 'bg-text-muted';
  }
}

export function shortPath(filePath: string, segments: number = 1): string {
  const parts = filePath.split('/');
  if (parts.length <= segments) return filePath;
  if (segments === 1) return parts[parts.length - 1];
  return `.../${parts.slice(-segments).join('/')}`;
}

export type ArtifactSummary = {
  totalCommits: number;
  pr: number | null;
  lastRunStatus: string | null;
  runCount: number;
};

export function parseArtifactSummary(body: string): ArtifactSummary | null {
  const artifactsIdx = body.indexOf('## Artifacts');
  if (artifactsIdx === -1) return null;

  const section = body.slice(artifactsIdx);
  const runHeaders = section.match(/### Run \d+ — (\w+)/g);
  if (!runHeaders || runHeaders.length === 0) return null;

  const lastStatus = runHeaders[runHeaders.length - 1].match(/— (\w+)/)?.[1] ?? null;

  let totalCommits = 0;
  const commitMatches = section.matchAll(/\*\*Commits:\*\* (\d+)/g);
  for (const m of commitMatches) {
    totalCommits += Number.parseInt(m[1], 10);
  }

  const prMatch = section.match(/\*\*PR:\*\* #(\d+)/);
  const pr = prMatch ? Number.parseInt(prMatch[1], 10) : null;

  return { totalCommits, pr, lastRunStatus: lastStatus, runCount: runHeaders.length };
}
