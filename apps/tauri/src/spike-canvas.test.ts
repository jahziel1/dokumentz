import { describe, expect, it } from 'vitest';
import { CELLS, GEOM, SCROLL, frameStats, runPhase } from './spike-canvas';

describe('geometría del spike', () => {
  it('pinta 2 800 celdas por cuadro', () => {
    expect(GEOM.cols * GEOM.rows).toBe(2800);
    expect(CELLS).toBe(2800);
  });
});

/** `requestAnimationFrame` falso: cuadros cada 16 ms de reloj simulado. */
function fakeLoop(frameMs: number) {
  let ts = 1000;
  let pending: FrameRequestCallback | null = null;
  const raf = (cb: FrameRequestCallback): number => {
    pending = cb;
    return 1;
  };
  const step = (): boolean => {
    const cb = pending;
    if (!cb) return false;
    pending = null;
    ts += frameMs;
    cb(ts);
    return true;
  };
  return { raf, step };
}

describe('runPhase', () => {
  it('descarta el calentamiento, mide el resto y desplaza con el tiempo', async () => {
    const calls: Array<[number, number]> = [];
    let flushes = 0;
    const painter = {
      paint: (x: number, y: number) => void calls.push([x, y]),
      flush: () => void flushes++,
    };
    const loop = fakeLoop(16);
    let clock = 0;
    const now = () => (clock += 2); // cada paint "dura" 2 ms: dos lecturas por cuadro
    const p = runPhase(painter, 160, 80, true, loop.raf, now);
    while (loop.step());
    const r = await p;
    expect(r.paint.length).toBeGreaterThan(0);
    expect(r.paint.every((v) => v === 2)).toBe(true);
    expect(r.interval.every((v) => v === 16)).toBe(true);
    expect(flushes).toBe(calls.length);
    expect(calls.length).toBeGreaterThan(r.paint.length); // el calentamiento también pinta
    const [x1, y1] = calls[1] as [number, number];
    expect(y1 / x1).toBeCloseTo(SCROLL.vy_px_s / SCROLL.vx_px_s);
    const st = frameStats(r);
    expect(st.paint_ms.p95_ms).toBe(2);
    expect(st.dropped_frames).toBe(0);
  });
});

describe('frameStats', () => {
  it('cuenta los cuadros perdidos (intervalo > 1,5 veces la mediana)', () => {
    const interval = Array.from({ length: 98 }, () => 16).concat([40, 50]);
    const st = frameStats({ paint: new Array(101).fill(1), interval, duration_ms: 1000 });
    expect(st.dropped_frames).toBe(2);
    expect(st.frames).toBe(101);
  });
});
