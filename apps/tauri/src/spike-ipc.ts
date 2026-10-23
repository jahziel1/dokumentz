import { Channel, invoke } from '@tauri-apps/api/core';
import { type Stats, summarize } from './spike-stats';

/** Tamaños de bloque del spike (bytes): 16 KB, 128 KB y 512 KB. */
export const SIZES: readonly number[] = [16 * 1024, 128 * 1024, 512 * 1024];

/** Compuerta CG-2 (plan.md §6.2): p95 de la ida y vuelta de 128 KB. */
export const CG2 = { size_bytes: 128 * 1024, p95_limit_ms: 10 } as const;

/** Valor recibido y tiempo (ms) hasta recibirlo. */
export interface Timed {
  value: unknown;
  ms: number;
}

/** Puerta de entrada al puente; en producción es Tauri, en las pruebas un doble. */
export interface IpcIo {
  /** `ipc_echo(n)`: ida y vuelta de `invoke` que devuelve `n` bytes. */
  echo(n: number): Promise<Timed>;
  /** `ipc_channel_push(n)`: desde `invoke` hasta que el `Channel` entrega el mensaje de `n` bytes. */
  push(n: number): Promise<Timed>;
}

export interface SizeResult extends Stats {
  size_bytes: number;
  /**
   * Tiempo total de las `n` iteraciones medidas con una sola pareja de lecturas del reloj y su
   * media: `performance.now()` va redondeado a 1 ms en WebKit, así que los percentiles por
   * muestra son escalones de 1 ms y la media del total es la única cifra con resolución real.
   */
  total_ms: number;
  mean_from_total_ms: number;
  /** Todas las respuestas fueron `ArrayBuffer`. */
  arraybuffer_ok: boolean;
  /** Todas tenían la longitud y el patrón esperados (`i % 251`). */
  bytes_ok: boolean;
}

export interface IpcReport {
  spike: 'ipc';
  task: 'T009';
  ok: boolean;
  iterations: number;
  warmup: number;
  /** `ipc_echo`: `invoke` con respuesta binaria. */
  echo: SizeResult[];
  /** `ipc_channel_push`: tiempo hasta que llega el mensaje por `Channel<InvokeResponseBody>`. */
  channel: SizeResult[];
  cg2: { size_bytes: number; p95_limit_ms: number; p95_ms: number; pass: boolean };
}

export interface IpcOptions {
  iterations: number;
  warmup: number;
  sizes: readonly number[];
  onProgress?: (msg: string) => void;
}

function pattern(i: number): number {
  return i % 251;
}

/** Comprueba tipo, longitud y tres bytes del patrón. */
export function checkBytes(v: unknown, n: number): { isBuffer: boolean; bytesOk: boolean } {
  if (!(v instanceof ArrayBuffer)) return { isBuffer: false, bytesOk: false };
  if (v.byteLength !== n) return { isBuffer: true, bytesOk: false };
  if (n === 0) return { isBuffer: true, bytesOk: true };
  const u8 = new Uint8Array(v);
  const mid = n >> 1;
  const ok = u8[0] === pattern(0) && u8[mid] === pattern(mid) && u8[n - 1] === pattern(n - 1);
  return { isBuffer: true, bytesOk: ok };
}

async function measure(
  call: (n: number) => Promise<Timed>,
  n: number,
  opts: IpcOptions,
): Promise<SizeResult> {
  for (let i = 0; i < opts.warmup; i++) await call(n);
  const samples: number[] = [];
  let arraybuffer_ok = true;
  let bytes_ok = true;
  const t0 = performance.now();
  for (let i = 0; i < opts.iterations; i++) {
    const t = await call(n);
    samples.push(t.ms);
    const c = checkBytes(t.value, n);
    arraybuffer_ok &&= c.isBuffer;
    bytes_ok &&= c.bytesOk;
  }
  const total_ms = performance.now() - t0;
  return {
    size_bytes: n,
    ...summarize(samples),
    total_ms: Math.round(total_ms * 1000) / 1000,
    mean_from_total_ms: Math.round((total_ms / opts.iterations) * 1e4) / 1e4,
    arraybuffer_ok,
    bytes_ok,
  };
}

export async function runIpcSpike(io: IpcIo, opts: IpcOptions): Promise<IpcReport> {
  const echo: SizeResult[] = [];
  const channel: SizeResult[] = [];
  for (const n of opts.sizes) {
    opts.onProgress?.(`ipc_echo ${n} B`);
    echo.push(await measure((k) => io.echo(k), n, opts));
    opts.onProgress?.(`channel ${n} B`);
    channel.push(await measure((k) => io.push(k), n, opts));
  }
  const at = echo.find((r) => r.size_bytes === CG2.size_bytes);
  const p95 = at?.p95_ms ?? Number.NaN;
  const all = [...echo, ...channel];
  return {
    spike: 'ipc',
    task: 'T009',
    ok: all.every((r) => r.arraybuffer_ok && r.bytes_ok),
    iterations: opts.iterations,
    warmup: opts.warmup,
    echo,
    channel,
    cg2: { ...CG2, p95_ms: p95, pass: p95 <= CG2.p95_limit_ms },
  };
}

/** Espera máxima por un mensaje del `Channel`; si vence, el spike falla en vez de colgarse. */
const CHANNEL_TIMEOUT_MS = 10_000;

/** Implementación real sobre `invoke` y `Channel` de `@tauri-apps/api`. */
export function createTauriIpcIo(now: () => number = () => performance.now()): IpcIo {
  return {
    async echo(n) {
      const t0 = now();
      const value = await invoke<ArrayBuffer>('ipc_echo', { n });
      return { value, ms: now() - t0 };
    },
    async push(n) {
      // Un `Channel` de JavaScript por llamada: en Rust cada invocación crea su propio canal con
      // el índice de mensajes en 0, y al terminar el comando envía el cierre que libera el
      // callback. Crear el canal cuenta dentro del tiempo medido (es parte de la petición).
      const t0 = now();
      let timer: ReturnType<typeof setTimeout> | undefined;
      const arrived = new Promise<unknown>((resolve, reject) => {
        const channel = new Channel<ArrayBuffer>(resolve);
        timer = setTimeout(
          () => reject(new Error(`el Channel no entregó ${n} B en ${CHANNEL_TIMEOUT_MS} ms`)),
          CHANNEL_TIMEOUT_MS,
        );
        invoke<void>('ipc_channel_push', { n, onChunk: channel }).then(() => undefined, reject);
      });
      try {
        const value = await arrived;
        return { value, ms: now() - t0 };
      } finally {
        clearTimeout(timer);
      }
    },
  };
}
