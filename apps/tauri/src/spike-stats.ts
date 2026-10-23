/** Resumen estadístico de una serie de tiempos en milisegundos. */
export interface Stats {
  n: number;
  min_ms: number;
  p50_ms: number;
  p95_ms: number;
  p99_ms: number;
  max_ms: number;
  mean_ms: number;
}

/** Percentil por rango más cercano (nearest-rank) sobre una serie ya ordenada. */
export function percentile(sorted: readonly number[], p: number): number {
  if (sorted.length === 0) return Number.NaN;
  const rank = Math.ceil((p / 100) * sorted.length);
  const idx = Math.min(sorted.length - 1, Math.max(0, rank - 1));
  return sorted[idx] as number;
}

/** Redondea a 4 decimales (0,1 microsegundo) para que el JSON sea legible. */
function r4(x: number): number {
  return Math.round(x * 1e4) / 1e4;
}

export function summarize(samples: readonly number[]): Stats {
  const sorted = [...samples].sort((a, b) => a - b);
  const n = sorted.length;
  const sum = sorted.reduce((acc, v) => acc + v, 0);
  return {
    n,
    min_ms: r4(sorted[0] ?? Number.NaN),
    p50_ms: r4(percentile(sorted, 50)),
    p95_ms: r4(percentile(sorted, 95)),
    p99_ms: r4(percentile(sorted, 99)),
    max_ms: r4(sorted[n - 1] ?? Number.NaN),
    mean_ms: r4(n === 0 ? Number.NaN : sum / n),
  };
}

/**
 * Resolución efectiva de `performance.now()`: la menor diferencia positiva entre lecturas
 * consecutivas (WebKit redondea el reloj en páginas sin aislamiento de origen cruzado).
 */
export function timerResolutionMs(now: () => number = () => performance.now(), spins = 200_000): number {
  let best = Number.POSITIVE_INFINITY;
  let prev = now();
  for (let i = 0; i < spins; i++) {
    const t = now();
    const d = t - prev;
    if (d > 0 && d < best) best = d;
    prev = t;
  }
  return Number.isFinite(best) ? r4(best) : Number.NaN;
}

/** Información del entorno que acompaña a cada informe. */
export interface Env {
  user_agent: string;
  device_pixel_ratio: number;
  screen: string;
  inner: string;
  cross_origin_isolated: boolean;
  timer_resolution_ms: number;
}

export function captureEnv(): Env {
  return {
    user_agent: navigator.userAgent,
    device_pixel_ratio: window.devicePixelRatio,
    screen: `${screen.width}x${screen.height}`,
    inner: `${window.innerWidth}x${window.innerHeight}`,
    cross_origin_isolated: window.crossOriginIsolated,
    timer_resolution_ms: timerResolutionMs(),
  };
}
