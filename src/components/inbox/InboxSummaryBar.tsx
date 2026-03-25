import { createMemo } from 'solid-js';
import { getInboxStore } from '../../lib/stores/inbox';
import type { SummaryStatItem } from '../../types/ui';
import { SummaryStatsBar } from '../ui/SummaryStatsBar';

export function InboxSummaryBar() {
  const inbox = getInboxStore();

  const stats = createMemo((): SummaryStatItem[] => {
    const all = inbox.items();
    if (all.length === 0) return [];

    const prCount = all.filter((i) => i.type === 'pr').length;
    const satFindings = all.filter((i) => i.source === 'sat').length;
    const completed = all.filter((i) => i.status === 'completed').length;

    const totalDelta = all.reduce((sum, i) => sum + (i.satDelta ?? 0), 0);

    const result: SummaryStatItem[] = [
      { label: 'PRs', value: String(prCount), color: 'primary' },
      { label: 'SAT findings', value: String(satFindings) },
    ];

    if (totalDelta !== 0) {
      result.push({
        label: 'Satisfaction',
        value: `${totalDelta > 0 ? '+' : ''}${totalDelta} pts`,
        color: totalDelta > 0 ? 'success' : 'error',
      });
    }

    if (completed > 0) {
      result.push({ label: 'Resolved', value: String(completed), color: 'success' });
    }

    return result;
  });

  return <SummaryStatsBar stats={stats()} />;
}
