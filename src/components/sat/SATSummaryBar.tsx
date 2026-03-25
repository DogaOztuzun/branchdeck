import { createMemo } from 'solid-js';
import { getSATStore } from '../../lib/stores/sat';
import type { SummaryStatItem } from '../../types/ui';
import { SummaryStatsBar } from '../ui/SummaryStatsBar';

export function SATSummaryBar() {
  const sat = getSATStore();

  const stats = createMemo((): SummaryStatItem[] => {
    const score = sat.currentScore();
    const delta = sat.scoreDelta();
    const sq = sat.signalQuality();
    const c = sat.cycles();

    const result: SummaryStatItem[] = [{ label: 'Score', value: String(score), color: 'primary' }];

    if (delta !== 0) {
      result.push({
        label: '',
        value: `${delta > 0 ? '+' : ''}${delta} pts`,
        color: delta > 0 ? 'success' : 'error',
      });
    }

    result.push({
      label: 'Signal',
      value: `${sq}%`,
      color: sat.signalColor(),
    });

    if (c.length > 0) {
      result.push({ label: 'Cycle', value: String(c.length) });
    }

    return result;
  });

  return <SummaryStatsBar stats={stats()} />;
}
