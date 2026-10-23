import { type Stats, summarize } from './spike-stats';

/** Geometría del spike (T010): 50 x 56 = 2 800 celdas pintadas en cada cuadro. */
export const GEOM = { cols: 50, rows: 56, cell_w: 28, cell_h: 14, font_px: 10, pad: 3 } as const;
export const CELLS = GEOM.cols * GEOM.rows;
export const CSS_W = GEOM.cols * GEOM.cell_w; // 1400
export const CSS_H = GEOM.rows * GEOM.cell_h; // 784

/** Desplazamiento sintético (px CSS por segundo): diagonal, siempre con celdas nuevas. */
export const SCROLL = { vx_px_s: 240, vy_px_s: 480 } as const;

/** Presupuesto de pintado por encima del cual se considera WebGL (plan.md §6.5, D-09). */
export const WEBGL_SIGNAL_MS = 6;

const FONT_REG = `${GEOM.font_px}px system-ui, sans-serif`;
const FONT_BOLD = `bold ${GEOM.font_px}px system-ui, sans-serif`;
const FILLS = ['', '#dbe7fb', '#ececec'] as const;

const SHORT = ['Q1', 'Q2', 'Q3', 'Q4', 'Sí', 'No', 'Sur', 'Ene', 'Feb', 'Mar'];
const LONG = ['Ingresos netos 2026', 'Concepto largo de prueba', 'Región Norte (provisional)'];
const NUMS = Array.from({ length: 1000 }, (_, i) => String(i));
const BIG = ['1234567', '98765432', '12345.678']; // no caben: se pintan como "####"

/** Mezcla barata y determinista de (fila, columna) absolutas. */
function hash(r: number, c: number): number {
  return (Math.imul(r + 0x9e37, 73856093) ^ Math.imul(c + 0x85eb, 19349663)) >>> 0;
}

function fillOf(r: number, c: number): number {
  if (r % 10 === 0) return 2;
  return (r + c) % 9 === 0 ? 1 : 0;
}

export interface PaintCounters {
  frames_painted: number;
  measure_calls: number;
  cache_hits: number;
  clip_calls: number;
  hash_cells: number;
}

function newCounters(): PaintCounters {
  return { frames_painted: 0, measure_calls: 0, cache_hits: 0, clip_calls: 0, hash_cells: 0 };
}

/** Pintor de la cuadrícula: relleno por color, líneas en un solo trazo y texto con caché de `measureText`. */
export class GridPainter {
  readonly canvas: HTMLCanvasElement;
  private readonly ctx: CanvasRenderingContext2D;
  private readonly widthReg = new Map<string, number>();
  private readonly widthBold = new Map<string, number>();
  counters: PaintCounters = newCounters();
  readonly dpr: number;

  constructor(doc: Document, dpr: number) {
    this.dpr = dpr;
    this.canvas = doc.createElement('canvas');
    this.canvas.width = Math.round(CSS_W * dpr);
    this.canvas.height = Math.round(CSS_H * dpr);
    this.canvas.style.width = `${CSS_W}px`;
    this.canvas.style.height = `${CSS_H}px`;
    const ctx = this.canvas.getContext('2d', { alpha: false });
    if (!ctx) throw new Error('canvas 2D no disponible');
    this.ctx = ctx;
  }

  resetCounters(): void {
    this.counters = newCounters();
  }

  get cacheEntries(): number {
    return this.widthReg.size + this.widthBold.size;
  }

  private width(cache: Map<string, number>, text: string): number {
    const hit = cache.get(text);
    if (hit !== undefined) {
      this.counters.cache_hits++;
      return hit;
    }
    this.counters.measure_calls++;
    const w = this.ctx.measureText(text).width;
    cache.set(text, w);
    return w;
  }

  /** Pinta un cuadro con el origen del desplazamiento en (`scrollX`, `scrollY`) px CSS. */
  paint(scrollX: number, scrollY: number): void {
    const ctx = this.ctx;
    this.counters.frames_painted++;
    const { cols, rows, cell_w: cw, cell_h: ch, pad } = GEOM;
    const sx = Math.round(scrollX);
    const sy = Math.round(scrollY);
    const c0 = Math.floor(sx / cw);
    const r0 = Math.floor(sy / ch);
    const ox = -(sx - c0 * cw);
    const oy = -(sy - r0 * ch);

    ctx.setTransform(this.dpr, 0, 0, this.dpr, 0, 0);
    ctx.fillStyle = '#ffffff';
    ctx.fillRect(0, 0, CSS_W, CSS_H);

    // Relleno agrupado por color.
    for (let f = 1; f < FILLS.length; f++) {
      ctx.fillStyle = FILLS[f] as string;
      for (let r = 0; r < rows; r++) {
        const y = oy + r * ch;
        for (let c = 0; c < cols; c++) {
          if (fillOf(r0 + r, c0 + c) === f) ctx.fillRect(ox + c * cw, y, cw, ch);
        }
      }
    }

    // Cuadrícula: líneas de 1 px alineadas a 0,5 en un solo trazo.
    ctx.strokeStyle = '#d4d4d4';
    ctx.lineWidth = 1;
    ctx.beginPath();
    for (let c = 0; c <= cols; c++) {
      const x = Math.round(ox + c * cw) + 0.5;
      ctx.moveTo(x, 0);
      ctx.lineTo(x, CSS_H);
    }
    for (let r = 0; r <= rows; r++) {
      const y = Math.round(oy + r * ch) + 0.5;
      ctx.moveTo(0, y);
      ctx.lineTo(CSS_W, y);
    }
    ctx.stroke();

    // Texto.
    ctx.fillStyle = '#111111';
    ctx.textBaseline = 'middle';
    let font = '';
    let align: CanvasTextAlign | '' = '';
    const room = cw - 2 * pad;
    for (let r = 0; r < rows; r++) {
      const ar = r0 + r;
      const bold = ar % 10 === 0;
      const wantFont = bold ? FONT_BOLD : FONT_REG;
      if (wantFont !== font) {
        ctx.font = wantFont;
        font = wantFont;
      }
      const cache = bold ? this.widthBold : this.widthReg;
      const y = oy + r * ch + ch / 2 + 0.5;
      for (let c = 0; c < cols; c++) {
        const ac = c0 + c;
        const h = hash(ar, ac);
        const isText = ac % 4 === 0;
        let text: string;
        if (isText) {
          text = h % 12 === 0 ? (LONG[h % LONG.length] as string) : (SHORT[h % SHORT.length] as string);
        } else {
          text = h % 40 === 0 ? (BIG[h % BIG.length] as string) : (NUMS[h % 1000] as string);
        }
        const x0 = ox + c * cw;
        const w = this.width(cache, text);
        if (w <= room) {
          const wantAlign = isText ? 'left' : 'right';
          if (wantAlign !== align) {
            ctx.textAlign = wantAlign;
            align = wantAlign;
          }
          ctx.fillText(text, isText ? x0 + pad : x0 + cw - pad, y);
        } else if (!isText) {
          // Un número que no cabe se pinta como "####" (EX-01).
          this.counters.hash_cells++;
          if (align !== 'right') {
            ctx.textAlign = 'right';
            align = 'right';
          }
          this.width(cache, '####'); // el ancho de "####" también entra en la caché
          ctx.fillText('####', x0 + cw - pad, y);
        } else {
          // Un texto que no cabe y no puede desbordar se recorta a la celda.
          this.counters.clip_calls++;
          ctx.save();
          ctx.beginPath();
          ctx.rect(x0, oy + r * ch, cw, ch);
          ctx.clip();
          ctx.textAlign = 'left';
          ctx.fillText(text, x0 + pad, y);
          ctx.restore(); // devuelve la fuente y la alineación que ya se seguían
        }
      }
    }
  }

  /** Fuerza a que el motor termine de rasterizar (lectura de un píxel); solo para la fase de diagnóstico. */
  flush(): void {
    this.ctx.getImageData(0, 0, 1, 1);
  }
}

export interface PhaseResult {
  paint: number[];
  interval: number[];
  duration_ms: number;
}

/** Corre cuadros con `requestAnimationFrame`: `warmup_ms` descartados y luego `duration_ms` medidos. */
export function runPhase(
  painter: Pick<GridPainter, 'paint' | 'flush'>,
  duration_ms: number,
  warmup_ms: number,
  flush: boolean,
  raf: (cb: FrameRequestCallback) => number = (cb) => requestAnimationFrame(cb),
  now: () => number = () => performance.now(),
): Promise<PhaseResult> {
  return new Promise((resolve) => {
    const paint: number[] = [];
    const interval: number[] = [];
    let start = -1;
    let prev = -1;
    let first = -1;
    const tick = (ts: number): void => {
      if (start < 0) start = ts;
      const elapsed = ts - start;
      const secs = elapsed / 1000;
      const t0 = now();
      painter.paint(secs * SCROLL.vx_px_s, secs * SCROLL.vy_px_s);
      if (flush) painter.flush();
      const t1 = now();
      if (elapsed >= warmup_ms) {
        if (first < 0) first = ts;
        paint.push(t1 - t0);
        if (prev >= 0) interval.push(ts - prev);
        prev = ts;
      }
      if (elapsed < warmup_ms + duration_ms) raf(tick);
      else resolve({ paint, interval, duration_ms: ts - first });
    };
    raf(tick);
  });
}

export interface FrameStats {
  frames: number;
  fps: number;
  paint_ms: Stats;
  frame_interval_ms: Stats;
  /** Intervalos entre cuadros mayores que 1,5 veces la mediana (cuadros perdidos). */
  dropped_frames: number;
}

export function frameStats(p: PhaseResult): FrameStats {
  const interval = summarize(p.interval);
  const limit = interval.p50_ms * 1.5;
  return {
    frames: p.paint.length,
    fps: Math.round((p.paint.length / (p.duration_ms / 1000)) * 100) / 100,
    paint_ms: summarize(p.paint),
    frame_interval_ms: interval,
    dropped_frames: p.interval.filter((d) => d > limit).length,
  };
}

export interface CanvasReport {
  spike: 'canvas';
  task: 'T010';
  ok: boolean;
  geometry: typeof GEOM & { cells: number; canvas_css: string; canvas_px: string; dpr: number };
  scroll: typeof SCROLL;
  duration_s: number;
  warmup_s: number;
  /** Fase principal: tiempo de JavaScript de `paint()` y del intervalo entre `requestAnimationFrame`. */
  measured: FrameStats;
  text: PaintCounters & { cache_entries: number };
  /** Diagnóstico: igual, pero con una lectura de píxel tras cada cuadro (incluye la rasterización pendiente). */
  flushed: FrameStats & { duration_s: number; note: string };
  webgl_signal: { threshold_ms: number; p95_paint_ms: number; exceeds: boolean; p95_paint_flushed_ms: number; exceeds_flushed: boolean };
}

export interface CanvasOptions {
  duration_ms: number;
  warmup_ms: number;
  flush_duration_ms: number;
}

export async function runCanvasSpike(
  stage: HTMLElement,
  opts: CanvasOptions,
  dpr: number = window.devicePixelRatio,
): Promise<CanvasReport> {
  const painter = new GridPainter(document, dpr);
  stage.replaceChildren(painter.canvas);

  // Calentamiento de caché y JIT: no cuenta.
  await runPhase(painter, 0, 500, false);
  painter.resetCounters();
  const main = await runPhase(painter, opts.duration_ms, opts.warmup_ms, false);
  const counters = painter.counters;
  const entries = painter.cacheEntries;
  const flushedPhase = await runPhase(painter, opts.flush_duration_ms, 500, true);

  const measured = frameStats(main);
  const flushed = frameStats(flushedPhase);
  return {
    spike: 'canvas',
    task: 'T010',
    ok: measured.frames > 0 && flushed.frames > 0,
    geometry: {
      ...GEOM,
      cells: CELLS,
      canvas_css: `${CSS_W}x${CSS_H}`,
      canvas_px: `${painter.canvas.width}x${painter.canvas.height}`,
      dpr,
    },
    scroll: SCROLL,
    duration_s: opts.duration_ms / 1000,
    warmup_s: opts.warmup_ms / 1000,
    measured,
    text: { ...counters, cache_entries: entries },
    flushed: {
      ...flushed,
      duration_s: opts.flush_duration_ms / 1000,
      note: 'diagnostico: getImageData(0,0,1,1) tras cada cuadro fuerza la rasterizacion; no es la medicion principal',
    },
    webgl_signal: {
      threshold_ms: WEBGL_SIGNAL_MS,
      p95_paint_ms: measured.paint_ms.p95_ms,
      exceeds: measured.paint_ms.p95_ms > WEBGL_SIGNAL_MS,
      p95_paint_flushed_ms: flushed.paint_ms.p95_ms,
      exceeds_flushed: flushed.paint_ms.p95_ms > WEBGL_SIGNAL_MS,
    },
  };
}
