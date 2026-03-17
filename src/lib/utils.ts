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
