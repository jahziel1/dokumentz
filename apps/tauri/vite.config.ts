import { defineConfig } from 'vitest/config';

// Tauri espera un puerto fijo en desarrollo y la carpeta `dist` en producción.
export default defineConfig({
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  build: { target: 'es2022', outDir: 'dist', emptyOutDir: true },
  test: { include: ['src/**/*.test.ts'], environment: 'node' },
});
