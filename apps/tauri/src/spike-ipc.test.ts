import { describe, expect, it } from 'vitest';
import { CG2, checkBytes, type IpcIo, runIpcSpike, type Timed } from './spike-ipc';

function bytes(n: number): ArrayBuffer {
  const u8 = new Uint8Array(n);
  for (let i = 0; i < n; i++) u8[i] = i % 251;
  return u8.buffer;
}

/** Doble del puente: devuelve bytes correctos con un tiempo fijo por tamaño. */
function fakeIo(msFor: (n: number) => number, corrupt = false): IpcIo {
  const reply = async (n: number): Promise<Timed> => ({
    value: corrupt ? new Uint8Array(n).fill(7).buffer : bytes(n),
    ms: msFor(n),
  });
  return { echo: reply, push: reply };
}

describe('checkBytes', () => {
  it('acepta un ArrayBuffer con el patrón', () => {
    expect(checkBytes(bytes(1000), 1000)).toEqual({ isBuffer: true, bytesOk: true });
  });
  it('rechaza longitud errónea, patrón roto y tipos que no son ArrayBuffer', () => {
    expect(checkBytes(bytes(10), 11)).toEqual({ isBuffer: true, bytesOk: false });
    expect(checkBytes(new Uint8Array(10).fill(9).buffer, 10).bytesOk).toBe(false);
    expect(checkBytes([1, 2, 3], 3)).toEqual({ isBuffer: false, bytesOk: false });
    expect(checkBytes(new Uint8Array(3), 3).isBuffer).toBe(false);
  });
});

describe('runIpcSpike', () => {
  const opts = { iterations: 20, warmup: 2, sizes: [16 * 1024, CG2.size_bytes, 512 * 1024] };

  it('CG-2 pasa si el p95 de 128 KB es <= 10 ms', async () => {
    const r = await runIpcSpike(fakeIo(() => 3), opts);
    expect(r.ok).toBe(true);
    expect(r.echo.map((e) => e.size_bytes)).toEqual(opts.sizes);
    expect(r.channel).toHaveLength(3);
    expect(r.cg2).toMatchObject({ p95_ms: 3, pass: true });
    expect(r.echo[0]?.total_ms).toBeGreaterThanOrEqual(0);
    expect(r.echo[0]?.mean_from_total_ms).toBeGreaterThanOrEqual(0);
  });

  it('CG-2 falla si el p95 de 128 KB supera 10 ms', async () => {
    const r = await runIpcSpike(fakeIo((n) => (n === CG2.size_bytes ? 12 : 1)), opts);
    expect(r.cg2.pass).toBe(false);
    expect(r.cg2.p95_ms).toBe(12);
  });

  it('marca el informe como no válido si llegan bytes corruptos', async () => {
    const r = await runIpcSpike(fakeIo(() => 1, true), opts);
    expect(r.ok).toBe(false);
    expect(r.echo[0]?.arraybuffer_ok).toBe(true);
    expect(r.echo[0]?.bytes_ok).toBe(false);
  });
});
