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
    const fpRate = sat.falsePositiveRate();
    const acc = sat.classificationAccuracy();

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

    if (fpRate !== null) {
      const fpTrend = sat.falsePositiveRateTrend();
      result.push({
        label: 'FP Rate',
        value: `${fpRate}%`,
        color: fpRate <= 10 ? 'success' : fpRate <= 25 ? 'warning' : 'error',
        sparkline:
          fpTrend.length >= 2 ? { data: fpTrend.map((t) => t.rate), color: '#f7768e' } : undefined,
      });
    }

    if (acc.accuracy !== null) {
      const accTrend = sat.classificationAccuracyTrend();
      result.push({
        label: 'Accuracy',
        value: `${acc.accuracy}%`,
        color: acc.accuracy >= 80 ? 'success' : acc.accuracy >= 60 ? 'warning' : 'error',
        sparkline:
          accTrend.length >= 2
            ? { data: accTrend.map((t) => t.accuracy), color: '#9ece6a' }
            : undefined,
      });
    }

    if (c.length > 0) {
      result.push({ label: 'Cycle', value: String(c.length) });
    }

    return result;
  });

  return <SummaryStatsBar stats={stats()} />;
}
