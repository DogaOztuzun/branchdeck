import { beforeEach, describe, expect, it } from 'vitest';
import { getSATStore } from '../stores/sat';

describe('SAT store confidence and metrics', () => {
  const sat = getSATStore();

  beforeEach(() => {
    sat.loadMockData();
  });

  describe('confidenceLevel', () => {
    it('returns high for confidence >= 75', () => {
      expect(sat.confidenceLevel(75)).toBe('high');
      expect(sat.confidenceLevel(100)).toBe('high');
      expect(sat.confidenceLevel(92)).toBe('high');
    });

    it('returns medium for confidence 50-74', () => {
      expect(sat.confidenceLevel(50)).toBe('medium');
      expect(sat.confidenceLevel(74)).toBe('medium');
    });

    it('returns low for confidence < 50', () => {
      expect(sat.confidenceLevel(49)).toBe('low');
      expect(sat.confidenceLevel(0)).toBe('low');
      expect(sat.confidenceLevel(32)).toBe('low');
    });
  });

  describe('falsePositiveRate', () => {
    it('computes rate from mock data', () => {
      const rate = sat.falsePositiveRate();
      // Mock data: 4 FP out of 16 total = 25%
      expect(rate).toBe(25);
    });
  });

  describe('classificationAccuracy', () => {
    it('computes accuracy from mock data', () => {
      const acc = sat.classificationAccuracy();
      // Mock: TP=9, FP=4, accuracy = 9/(9+4) = 69%
      expect(acc.totalClassifications).toBe(16);
      expect(acc.truePositives).toBe(9);
      expect(acc.falsePositives).toBe(4);
      expect(acc.accuracy).toBe(69);
      expect(acc.cyclesCounted).toBe(4);
    });
  });

  describe('falsePositiveRateTrend', () => {
    it('returns per-cycle FP rates', () => {
      const trend = sat.falsePositiveRateTrend();
      expect(trend.length).toBe(4);
      // Cycle 1: 2/5 = 40%
      expect(trend[0]).toEqual({ cycle: 1, rate: 40 });
      // Cycle 3: 0/3 = 0%
      expect(trend[2]).toEqual({ cycle: 3, rate: 0 });
    });
  });

  describe('classificationAccuracyTrend', () => {
    it('returns per-cycle accuracy', () => {
      const trend = sat.classificationAccuracyTrend();
      // All cycles have issuesFixed + falsePositives > 0 except cycle 3 (0 FP, 2 fixed = denominator 2)
      expect(trend.length).toBeGreaterThan(0);
      // Cycle 2: 3/(3+1) = 75%
      const c2 = trend.find((t) => t.cycle === 2);
      expect(c2?.accuracy).toBe(75);
    });
  });
});
