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

  // Sort all groups: newest first by createdAt, failing CI prioritized
  const sortByRecent = (a: TriagePr, b: TriagePr) => {
    // Failing CI first
    const aFailing = a.pr?.ciStatus === 'FAILURE' || a.pr?.ciStatus === 'ERROR';
    const bFailing = b.pr?.ciStatus === 'FAILURE' || b.pr?.ciStatus === 'ERROR';
    if (aFailing && !bFailing) return -1;
    if (!aFailing && bFailing) return 1;
    // Then newest first
    const aTime = a.pr?.createdAt ? new Date(a.pr.createdAt).getTime() : 0;
    const bTime = b.pr?.createdAt ? new Date(b.pr.createdAt).getTime() : 0;
    return bTime - aTime;
  };

  newPrs.sort(sortByRecent);
  inProgress.sort(sortByRecent);
  watching.sort(sortByRecent);
  done.sort(sortByRecent);

  return { needsAttention, inProgress, watching, newPrs, done };
}
