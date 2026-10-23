import { describe, expect, it } from 'vitest';
import { percentile, summarize, timerResolutionMs } from './spike-stats';

describe('percentile (rango más cercano)', () => {
  const s = Array.from({ length: 1000 }, (_, i) => i + 1); // 1..1000
  it('p50, p95 y p99 de 1..1000', () => {
    expect(percentile(s, 50)).toBe(500);
    expect(percentile(s, 95)).toBe(950);
    expect(percentile(s, 99)).toBe(990);
  });
  it('extremos y serie vacía', () => {
    expect(percentile(s, 0)).toBe(1);
    expect(percentile(s, 100)).toBe(1000);
    expect(percentile([], 50)).toBeNaN();
    expect(percentile([7], 99)).toBe(7);
  });
});

describe('summarize', () => {
  it('ordena la serie y no la modifica', () => {
    const input = [5, 1, 3, 2, 4];
    const st = summarize(input);
    expect(input).toEqual([5, 1, 3, 2, 4]);
    expect(st).toMatchObject({ n: 5, min_ms: 1, p50_ms: 3, max_ms: 5, mean_ms: 3 });
  });
  it('serie vacía da NaN, no excepción', () => {
    expect(summarize([]).n).toBe(0);
    expect(summarize([]).p95_ms).toBeNaN();
  });
});

describe('timerResolutionMs', () => {
  it('detecta el escalón de un reloj redondeado a 1 ms', () => {
    let t = 0;
    const fake = () => {
      t += 0.25;
      return Math.floor(t);
    };
    expect(timerResolutionMs(fake, 1000)).toBe(1);
  });
});
