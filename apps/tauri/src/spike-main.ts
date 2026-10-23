import { invoke } from '@tauri-apps/api/core';
import './spike.css';
import { runCanvasSpike } from './spike-canvas';
import { SIZES, createTauriIpcIo, runIpcSpike } from './spike-ipc';
import { captureEnv } from './spike-stats';

/** Respuesta del comando `spike_args` (Rust). */
interface SpikeInfo {
  spike: 'ipc' | 'canvas' | 'all' | null;
  exit: boolean;
  appVersion: string;
  tauriVersion: string;
  os: string;
  arch: string;
}

type Kind = 'ipc' | 'canvas';

const ITERATIONS = 1000;
const WARMUP = 100;
const CANVAS = { duration_ms: 10_000, warmup_ms: 1_000, flush_duration_ms: 5_000 };

const statusEl = document.getElementById('status') as HTMLElement;
const logEl = document.getElementById('log') as HTMLElement;
const stageEl = document.getElementById('stage') as HTMLElement;

/**
 * Avisos de consola y violaciones de CSP vistos durante la sesión. El aviso «IPC custom protocol
 * failed» de Tauri indica que el protocolo `ipc` falló (p. ej. por la CSP) y cayó a `postMessage`.
 */
const observed: string[] = [];
for (const level of ['warn', 'error'] as const) {
  const original = console[level].bind(console);
  console[level] = (...args: unknown[]) => {
    observed.push(`${level}: ${args.map(String).join(' ')}`.slice(0, 300));
    original(...args);
  };
}
document.addEventListener('securitypolicyviolation', (e) => {
  observed.push(`csp: ${e.violatedDirective} bloqueó ${e.blockedURI}`.slice(0, 300));
});

function status(msg: string): void {
  statusEl.textContent = msg;
  logEl.textContent += `${msg}\n`;
}

/** Entrega el informe a Rust, que lo imprime en stdout y, si procede, cierra el proceso. */
async function send(payload: object, done: boolean, ok: boolean): Promise<void> {
  await invoke('spike_report', { json: JSON.stringify(payload), done, ok });
}

async function measureOne(kind: Kind, info: SpikeInfo): Promise<{ report: object; ok: boolean }> {
  const meta = {
    app_version: info.appVersion,
    tauri_version: info.tauriVersion,
    os: info.os,
    arch: info.arch,
    env: captureEnv(),
    csp: {
      violations_and_console_problems: [...observed],
      ipc_fell_back_to_postmessage: observed.some((m) => m.includes('IPC custom protocol failed')),
    },
  };
  if (kind === 'ipc') {
    const report = await runIpcSpike(createTauriIpcIo(), {
      iterations: ITERATIONS,
      warmup: WARMUP,
      sizes: SIZES,
      onProgress: status,
    });
    return { report: { ...report, meta }, ok: report.ok };
  }
  status('canvas: 10 s de desplazamiento sintético y 5 s de diagnóstico');
  const report = await runCanvasSpike(stageEl, CANVAS);
  return { report: { ...report, meta }, ok: report.ok };
}

async function runKinds(kinds: Kind[], info: SpikeInfo, exitAtEnd: boolean): Promise<void> {
  for (const [i, kind] of kinds.entries()) {
    const last = i === kinds.length - 1;
    try {
      status(`${kind}: midiendo`);
      const { report, ok } = await measureOne(kind, info);
      await send(report, last && exitAtEnd, ok);
      status(`${kind}: terminado`);
    } catch (e) {
      const error = e instanceof Error ? `${e.name}: ${e.message}` : String(e);
      await send({ spike: kind, ok: false, error }, exitAtEnd, false);
      status(`${kind}: ERROR ${error}`);
      return;
    }
  }
}

async function main(): Promise<void> {
  if (!('__TAURI_INTERNALS__' in window)) {
    status('Esta página necesita la ventana de Tauri (no hay puente IPC).');
    return;
  }
  const info = await invoke<SpikeInfo>('spike_args');
  const manual = (kinds: Kind[]): void => {
    void runKinds(kinds, info, false);
  };
  document.getElementById('run-ipc')?.addEventListener('click', () => manual(['ipc']));
  document.getElementById('run-canvas')?.addEventListener('click', () => manual(['canvas']));
  if (info.spike) {
    const kinds: Kind[] = info.spike === 'all' ? ['ipc', 'canvas'] : [info.spike];
    await runKinds(kinds, info, info.exit);
  } else {
    status('listo: pulsa un botón o arranca con --spike ipc|canvas|all [--exit]');
  }
}

main().catch(async (e: unknown) => {
  const error = e instanceof Error ? `${e.name}: ${e.message}` : String(e);
  status(`ERROR ${error}`);
  try {
    await send({ spike: 'main', ok: false, error }, true, false);
  } catch {
    // Sin puente no hay a quién informar.
  }
});
