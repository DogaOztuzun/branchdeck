import type { TriageGroups, TriagePr } from '../../types/lifecycle';

export function groupTriagePrs(items: TriagePr[]): TriageGroups {
  const needsAttention: TriagePr[] = [];
  const inProgress: TriagePr[] = [];
  const watching: TriagePr[] = [];
  const newPrs: TriagePr[] = [];
  const done: TriagePr[] = [];

  for (const item of items) {
    const status = item.lifecycle?.status;

    if (status === 'reviewReady' || status === 'failed') {
      needsAttention.push(item);
    } else if (
      status === 'running' ||
      status === 'fixing' ||
      status === 'approved' ||
      status === 'retrying'
    ) {
      inProgress.push(item);
    } else if (status === 'stale') {
      watching.push(item);
    } else if (status === 'completed') {
      done.push(item);
    } else if (!item.lifecycle && item.pr) {
      newPrs.push(item);
    }
  }

  // Sort newPrs: failing CI first
  newPrs.sort((a, b) => {
    const aFailing = a.pr?.ciStatus === 'FAILURE' || a.pr?.ciStatus === 'ERROR';
    const bFailing = b.pr?.ciStatus === 'FAILURE' || b.pr?.ciStatus === 'ERROR';
    if (aFailing && !bFailing) return -1;
    if (!aFailing && bFailing) return 1;
    return 0;
  });

  return { needsAttention, inProgress, watching, newPrs, done };
}
