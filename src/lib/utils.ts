export function statusColor(status: string): string {
  switch (status) {
    case 'active':
      return 'bg-accent-success';
    case 'idle':
      return 'bg-accent-warning';
    case 'stopped':
      return 'bg-text-dim';
    default:
      return 'bg-text-dim';
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
};

export function parseArtifactSummary(body: string): ArtifactSummary | null {
  const match = /^## Artifacts$/m.exec(body);
  if (!match) return null;

  const section = body.slice(match.index);
  const runHeaders = section.match(/### Run \d+ — (\w+)/g);
  if (!runHeaders) return null;

  let totalCommits = 0;
  const commitMatches = section.matchAll(/\*\*Commits:\*\* (\d+)/g);
  for (const m of commitMatches) {
    totalCommits += Number.parseInt(m[1], 10);
  }

  const prMatch = section.match(/\*\*PR:\*\* #(\d+)/);
  const pr = prMatch ? Number.parseInt(prMatch[1], 10) : null;

  return { totalCommits, pr };
}
