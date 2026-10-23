# Tareas 001 — PoC de hoja de cálculo

| Campo | Valor |
|---|---|
| Estado | Borrador para revisión del agente principal y del dueño del proyecto |
| Fecha | 2026-10-22 |
| Entradas | `specs/constitucion.md` (v1.1.1), `specs/000-arquitectura-suite/arquitectura.md` (arquitectura de la suite y hoja de ruta; esta PoC es la fase 0a), `spec.md`, `research.md`, `plan.md` |
| IDs | `T001…T118` (tareas), `E01…E33` (etapas de agente), `CP-0…CP-8` (puntos de control), `CG-1`, `CG-2` (compuertas de decisión temprana); `FR`, `M`, `SC`, `A1…A4`, `US1…US5` como en `spec.md` |

## Cómo leer este documento

**Formato de tarea:** `- [ ] Txxx [P] [USn] [MIDE] [HW-…] [MANUAL] Qué se hace — ruta(s). **Verificación:** comando o comprobación.`

| Marca | Significado |
|---|---|
| `[P]` | Se puede hacer en paralelo con otras tareas `[P]` de su etapa o fase: archivos distintos y sin dependencia entre ellas |
| `[US1]…[US5]` | Historia de usuario a la que sirve. Las historias P1/P2 tienen tareas **para ambas interfaces** (Tauri y GPUI) |
| `[MIDE]` | Principio II de la constitución: la tarea toca la interfaz o el núcleo y adjunta un JSON de medición **antes y después** (instrumentación interna, FR-012) en `results/dev/` |
| `[HW-Win]`, `[HW-Linux]`, `[HW-Linux-NVIDIA]`, `[HW-120Hz]` | **Dependiente de hardware real** que hoy no existe (aclaración 2 de `spec.md`). Bloqueada hasta tener el equipo; una VM solo vale para pruebas funcionales y sus datos se marcan «solo funcional» |
| `[MANUAL]` | Lo hace una persona (permisos del sistema, IME, juicio visual) |
| `[BLOQUEADA: …]` | Espera una decisión del dueño del proyecto |

**Etapas para agentes.** Cada etapa cabe en un agente con **< 150k de contexto** (la estimación está en cada una). Indica qué archivos **lee** y cuáles **toca**; el agente no debe leer ni modificar nada más. Agente `trabajador` = Sonnet (código, contenido, depuración); `mecanico` = Haiku, esfuerzo bajo (pruebas, mediciones, inventarios); `persona` = el dueño del proyecto. Las etapas con el mismo prefijo de fase se ejecutan **en el orden indicado**, salvo las que declaran «paralela con».

**Puntos de control `CP-n`.** El agente principal, antes de dar la etapa por buena y de lanzar la siguiente: (1) **vuelve a correr** las verificaciones listadas (no se fía del «ya quedó» del agente); (2) **revisa el diff** — o, mientras no haya git, lee los archivos que la etapa declara haber tocado y comprueba que no se tocó nada más —, buscando lógica de interfaz en el núcleo (principio III), código de ONLYOFFICE (VII) y dependencias nuevas sin auditar (I); (3) decide si se continúa. Máximo **2 rondas de corrección** propias por etapa; lo que quede pendiente se dice con honestidad en el informe de la etapa.

**Reglas para todos los agentes.**

- Leer primero este `tasks.md` (solo su etapa) y las secciones de `plan.md` que la etapa cita; usar `grep` antes de leer archivos grandes; no explorar el repositorio entero.
- Cada funcionalidad nueva empieza con una prueba contra la API del núcleo, antes de tocar cualquier interfaz (principio III).
- No hacer `git commit` ni `git init` salvo que el encargo lo pida (la carpeta aún no es un repositorio; CP-0 pide al dueño la decisión).
- No copiar código de ONLYOFFICE; se puede adaptar código de los ejemplos de Zed (Apache-2.0) anotando la procedencia en el archivo.
- Comandos con la caché estable: `cargo test-core` es el alias definido en T001 (`test --workspace --exclude dokumentz-app-tauri --exclude dokumentz-app-gpui`).
- Nunca medir ni informar rendimiento con compilaciones `debug`.
- Las salidas de medición van a `results/…` y, si son de desarrollo, a `results/dev/` (no cuentan para SC).

## Mapa de etapas

| Etapa | Contenido | Agente | Tareas | Contexto est. | Depende de |
|---|---|---|---|---|---|
| E01 | Esqueleto del repositorio | trabajador | T001–T004 | ~40k | — |
| E02 | Spike de IronCalc (carga y API) | trabajador | T005–T006 | ~70k | E01 |
| E03 | Spike de GPUI | trabajador + persona | T007–T008 | ~90k | E01 (paralela con E02, E04) |
| E04 | Spike de Tauri y CI mínima | trabajador | T009–T011 | ~90k | E01 (paralela con E02, E03) |
| **CP-0** | | principal | | | E01–E04 |
| E05 | `dokumentz-metrics` y `dokumentz-script` | trabajador | T012–T014 | ~60k | CP-0 |
| E06 | Núcleo I: abrir, geometría, lista de dibujo, códec | trabajador | T015–T019 | ~110k | E05 |
| E07 | Núcleo II: selección y navegación | trabajador | T020–T022 | ~80k | E06 |
| E08 | Núcleo III: edición, idioma, historial, hojas, portapapeles, guardado | trabajador | T023–T028 | ~110k | E07 |
| E09 | Núcleo IV: fachada, instrumentación y bancos | trabajador | T029–T032 | ~70k | E08 |
| **CP-1** | | principal | | | E05–E09 |
| E10 | Generador A1–A3 | trabajador | T033–T035 | ~70k | CP-0 (paralela con E06–E09) |
| E11 | Comparador, juez y reproducibilidad | trabajador | T036–T038 | ~70k | E10 (T038), CP-0 |
| E12 | `bench-runner`: esquema, entorno, memoria | trabajador | T039–T041 | ~100k | E05 |
| E13 | `bench-runner`: captura, entrada, análisis | trabajador | T042–T045 | ~120k | E12 |
| E14 | `bench-report`: agregación, criterios, decisión | trabajador | T046–T049 | ~90k | E05, E12 |
| **CP-2** | | principal | | | E10–E14 y CP-1 |
| E15 | Tauri: backend | trabajador | T050–T051 | ~80k | CP-2 |
| E16 | Tauri: frontend I (decodificador, canvas, desplazamiento) | trabajador | T052–T054 | ~100k | E15 |
| E17 | Tauri: frontend II (entrada, barra, pestañas, guion) | trabajador | T055–T056 | ~100k | E16 |
| E18 | GPUI: cuadrícula | trabajador | T057–T059 | ~110k | CP-2 (paralela con E15–E17) |
| E19 | GPUI: entrada, barra, pestañas, guion | trabajador | T060–T061 | ~90k | E18 |
| E20 | Pruebas y medición interna de US1 | trabajador / mecanico | T062–T063 | ~70k | E17, E19 |
| **CP-3** | | principal | | | E15–E20 |
| E21 | Tauri: edición e IME | trabajador | T064–T066 | ~110k | CP-3 |
| E22 | GPUI: edición e IME | trabajador | T067–T069 | ~120k | CP-3 (paralela con E21) |
| E23 | Pruebas, medición interna y guion manual de US2 | trabajador / mecanico / persona | T070–T072 | ~80k | E21, E22 |
| **CP-4** | | principal | | | E21–E23 |
| E24 | Hojas, portapapeles y guardado en ambas | trabajador | T073–T076 | ~110k | CP-4 |
| E25 | Integridad y jueces | trabajador / mecanico / persona | T077–T080, T114 | ~90k | E24 |
| E33 | Compatibilidad del motor (SC-007) | trabajador (T115, T117, T118) + persona (T116) | T115–T118 | ~90k | CP-1, E11 (T036), E14 (T047), E25 (T114) |
| **CP-5** | | principal | | | E24, E25, E33 |
| E26 | CI completa | trabajador | T081–T082 | ~90k | CP-5 (puede empezar antes) |
| E27 | Runbook y preparación en macOS | trabajador + persona | T083–T085 | ~60k | CP-5 |
| E28 | Medición externa en macOS | mecanico | T086–T092 | ~60k | E27, T049 |
| **CP-6** | | principal | | | E26–E28 |
| E29 | Medición en Windows `[HW-Win]` | mecanico + persona | T093–T098 | ~60k | CP-6 y hardware |
| E30 | Medición en Linux `[HW-Linux]` | mecanico + persona | T099–T105 | ~70k | CP-6 y hardware |
| E31 | Consolidación | trabajador | T106–T107 | ~60k | E28 (+ E29, E30 si hay datos) |
| **CP-7** | | principal | | | E29–E31 |
| E32 | Cierre | mecanico / trabajador / persona | T108–T113 | ~60k | CP-7 |
| **CP-8** | | principal + dueño | | | E32 |

**Ruta crítica:** E01 → E02 (CG-1) → E05 → E06 → E07 → E08 → E09 → CP-1 → (Tauri y GPUI) → CP-3 → CP-4 → CP-5 → E28 → informe. E33 (SC-007) no frena la decisión de interfaz, pero su regla del motor puede condicionar la fase 1. **Paralelismos permitidos:** E02/E03/E04; E10–E14 con E06–E09 (no tocan `crates/dokumentz-sheet`); Tauri (E15–E17) con GPUI (E18–E19); E21 con E22. Dos etapas en paralelo **nunca** editan el mismo archivo ni el `Cargo.toml` raíz.

---

## Fase 0 — Preparación

### E01 — Esqueleto del repositorio
- **Agente:** trabajador (~40k). **Lee:** `plan.md` §2.1, §2.2, §4; `constitucion.md` principios I y III; `specs/000-arquitectura-suite/arquitectura.md` (nombres de crates y carpetas, qué crates de la base común llegan en la fase 1 y no se crean aquí). **Toca:** raíz (`Cargo.toml`, `rust-toolchain.toml`, `.cargo/config.toml`, `deny.toml`, `rustfmt.toml`, `.editorconfig`, `.gitignore`), `crates/*`, `tools/*`, `apps/gpui/{Cargo.toml,src/main.rs}`, `apps/tauri/src-tauri/{Cargo.toml,src/main.rs}` (solo código mínimo), `crates/dokumentz-sheet/tests/no_ui_deps.rs`.

- [x] T001 Crear `rust-toolchain.toml` (canal `1.99.0`, `rustfmt`, `clippy`), `Cargo.toml` raíz (espacio de trabajo con **todos** los miembros de `plan.md` §4, `resolver = "3"`, edición 2024, `[workspace.dependencies]` con `ironcalc = "=0.9.1"`, `ironcalc_base = "=0.9.1"`, `rust_xlsxwriter = "=0.101.0"`, `calamine = "=0.37.0"`, `[profile.release]` común: `opt-level = 3`, `lto = "thin"`, `codegen-units = 1`), `.cargo/config.toml` (alias `test-core = "test --workspace --exclude dokumentz-app-tauri --exclude dokumentz-app-gpui"`), `.gitignore` (`target/`, `node_modules/`, `testdata/` salvo `testdata/a4/manifest.toml`, `results/**/evidencia/*.mp4`, `results/**/privado/`), `.editorconfig`, `rustfmt.toml`, `LICENSE-MIT`, `LICENSE-APACHE` y `README.md` mínimo (qué es, licencia MIT OR Apache-2.0, enlace a `specs/`); `[workspace.package]` con `license = "MIT OR Apache-2.0"` y `repository = "https://github.com/jahziel1/dokumentz"`. **Verificación:** dentro de la carpeta `rustc --version` imprime `1.99.0`; `cargo metadata --no-deps --format-version 1 > /dev/null` sin error.
- [x] T002 [P] Crear los crates mínimos, cada uno con una función `version()` y una prueba trivial: `crates/dokumentz-sheet`, `crates/dokumentz-metrics`, `crates/dokumentz-script`, `tools/testdata-gen`, `tools/xlsx-diff`, `tools/compat-check` (T115 lo completa), `tools/bench-runner`, `tools/bench-report`, y los *stubs* sin dependencias de `apps/gpui` (`dokumentz-app-gpui`) y `apps/tauri/src-tauri` (`dokumentz-app-tauri`) para que el `Cargo.toml` raíz no se vuelva a editar en paralelo. Solo `dokumentz-sheet` declara `ironcalc` e `ironcalc_base`. **Verificación:** `cargo test-core` pasa; `cargo build --workspace --release` termina sin errores.
- [x] T003 [P] `deny.toml`: licencias permitidas (MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, Unicode-3.0, y otras permisivas que aparezcan con justificación escrita); **denegadas**: `AGPL-*`, `GPL-*`, `LGPL-*` (de forma permanente: el proyecto es MIT OR Apache-2.0; las bibliotecas LGPL del sistema enlazadas dinámicamente no son crates) y licencias desconocidas; fuentes permitidas: crates.io y `github.com/zed-industries`. **Verificación:** `cargo deny check licenses bans sources` sin errores; prueba negativa documentada en el informe de la etapa (una dependencia temporal con licencia AGPL hace fallar el comando; se revierte).
- [x] T004 [P] `crates/dokumentz-sheet/tests/no_ui_deps.rs`: ejecuta `cargo metadata --format-version 1` y falla si `dokumentz-sheet` alcanza (directa o transitivamente) cualquiera de `tauri`, `tauri-*`, `wry`, `tao`, `gpui`, `gpui_*`, `gtk`, `gtk4`, `webkit2gtk`, `winit`, `objc2-app-kit`, `cocoa`. **Verificación:** `cargo test -p dokumentz-sheet --test no_ui_deps` pasa; al añadir temporalmente `winit` a `dokumentz-sheet` falla (comprobado y revertido).

### E02 — Spike de IronCalc (carga y API)
- **Agente:** trabajador (~70k). **Lee:** `plan.md` §1 (hallazgos), §5.1–§5.9, Anexo A (IronCalc); `research.md` D-02; docs.rs de `UserModel`. **Toca:** `crates/dokumentz-sheet/Cargo.toml` (solo `[dev-dependencies]`), `crates/dokumentz-sheet/tests/spike_load.rs`, `crates/dokumentz-sheet/tests/spike_api.rs`, `research.md` (solo la tabla «Hechos sin verificar o pendientes» y una sección de resultados).

- [x] T005 [MIDE] **Compuerta CG-1.** `crates/dokumentz-sheet/tests/spike_load.rs` (`#[ignore]`): genera con `rust_xlsxwriter` un `.xlsx` de 100 000×20 (números, textos, fechas y estilos) en un directorio temporal; mide `ironcalc::import::load_from_xlsx` + `UserModel::from_model`, el uso de memoria del proceso tras cargar (`footprint -p <pid> -j` en macOS o `ps -o rss`), `save_to_xlsx` y la reapertura. **Verificación:** `cargo test -p dokumentz-sheet --release --test spike_load -- --ignored --nocapture` imprime un JSON con `load_ms`, `rss_mb`, `save_ms`, `reload_ms`; el agente principal copia los números a `research.md` y aplica **CG-1**: si abrir tarda > 10 s o el uso pasa de 2 GB se avisa al dueño **antes** de seguir (umbrales de aviso propuestos; riesgo R-05).
- [x] T006 Spike de API contra **0.9.1** en `crates/dokumentz-sheet/tests/spike_api.rs`, con una aserción por hallazgo (comentario `// HALLAZGO:`): (1) filas/columnas base 1; (2) unidad y valor por defecto de `get_row_height`/`get_column_width`; (3) formato del argumento `key` de `on_expand_selected_range`; (4) campos de `SelectedView`; (5) con `set_language("es")`, `=TRUE()` da error de nombre y una fórmula en inglés no se acepta; `set_language` no evalúa ni entra en el historial; (6) coste de `evaluate()` en un libro de 100 000×20 con 10 000 fórmulas dependientes de una celda (A2 sintético), con una edición completa; (7) `set_user_input` evalúa todo el libro y `pause_evaluation` lo evita (contra 0.9.1, no solo `main`); (8) `resolve_color` devuelve `#RRGGBB`; (9) si `get_model().workbook` es público para leer excepciones de tamaño; (10) si la carga informa partes no soportadas; (11) `paste_csv_string` con comillas y saltos de línea; (12) `Clipboard.csv` no es accesible. **Verificación:** `cargo test -p dokumentz-sheet --release --test spike_api -- --nocapture` verde; cada marca `[A VERIFICAR EN T006]` de `plan.md` se actualiza (pasa a `[VERIFICADO]` o se corrige el diseño) y el resumen queda en `research.md`.

### E03 — Spike de GPUI
- **Agente:** trabajador (~90k) para T007; persona para T008. **Lee:** `plan.md` §2.2 (fila GPUI), §7, Anexo A (GPUI); el ejemplo `crates/gpui/examples/input.rs` de Zed en el commit candidato. **Toca:** `Cargo.toml` raíz (**solo** `[patch.crates-io]` y la dependencia git de `gpui`/`gpui_platform` en `[workspace.dependencies]`), `apps/gpui/Cargo.toml`, `apps/gpui/src/main.rs`.

- [x] T007 Spike de GPUI: dependencia git de `gpui` y `gpui_platform` con `rev` fijo (candidato `9e84b066b55bc3609a30dd965075c12255b38b45`, o el más reciente de `main` que compile; características `font-kit`, `wayland`, `x11`); replicar en el `Cargo.toml` raíz los `[patch.crates-io]` de Zed que apliquen (`async-task`, `calloop`, `khronos-egl`…) con los `rev` de ese commit; ventana con un `Element` propio que pinta 2 000 `paint_quad` y 500 `ShapedLine`, y un cuadro de texto con `ElementInputHandler` adaptado de `input.rs` (cabecera con procedencia y licencia Apache-2.0). **Verificación:** `cargo build -p dokumentz-app-gpui --release` sin errores en macOS; `cargo tree -p dokumentz-app-gpui -e normal | grep -i -E "agpl|gpl"` vacío; el commit queda comentado en `Cargo.toml` y en `research.md`; `cargo deny check licenses` sobre la clausura anotado (primera lectura de terceros, R-12); `cargo build -p dokumentz-app-gpui -p dokumentz-app-tauri` resuelve en un solo `Cargo.lock` (si no, aplicar R-10).
- [ ] T008 [MANUAL] En macOS, probar el cuadro de texto del spike: tecla muerta (`´` + `a`), `ñ`, IME japonés (Hiragana) y dictado. **Verificación:** tabla de 4 filas (sí/no/parcial + observación) en `research.md`; no bloquea las etapas siguientes pero alimenta R-07.

### E04 — Spike de Tauri y CI mínima
- **Agente:** trabajador (~90k). **Lee:** `plan.md` §6.1–§6.4, §10, Anexo A (Tauri). **Toca:** `apps/tauri/**` (esqueleto del frontend y `src-tauri`), `.github/workflows/ci.yml`. **No** edita el `Cargo.toml` raíz.

- [x] T009 [MIDE] **Compuerta CG-2.** Spike del puente en `apps/tauri` (Vite + TypeScript sin framework, `src-tauri` con `tauri = "~2.12"`): comandos `async` `ipc_echo(n) -> tauri::ipc::Response` y un `Channel<InvokeResponseBody>` que empuja `n` bytes; el frontend mide 1 000 idas y vueltas para 16 KB, 128 KB y 512 KB (p50/p95/p99 con `performance.now()`) y comprueba que llega un `ArrayBuffer`. **Verificación:** `cd apps/tauri && npm ci && npm run tauri build -- --no-bundle` compila; el binario `release` imprime el JSON de latencias; **CG-2**: p95 de 128 KB ≤ 10 ms en el Mac, o se anota el rediseño del reparto de bloques antes de seguir (R-01); `package-lock.json` fija las versiones de Vite, TypeScript y Vitest.
- [x] T010 [MIDE] Spike de canvas 2D en el mismo esqueleto: una página que pinta en cada cuadro 2 800 celdas (relleno + texto con caché de `measureText`) durante 10 s de desplazamiento sintético y mide el tiempo de pintado y el intervalo entre `requestAnimationFrame`. **Verificación:** JSON con p95 de pintado y de intervalo; si el p95 de pintado > 6 ms se anota en `research.md` como señal para considerar WebGL (D-09), sin implementarlo.
- [x] T011 CI mínima: `.github/workflows/ci.yml` con el trabajo `core` de `plan.md` §10 en la matriz de tres SO y un trabajo que compila `dokumentz-app-tauri` y `dokumentz-app-gpui` (Linux con las listas de paquetes verificadas del plan). **Verificación:** `actionlint .github/workflows/ci.yml` sin errores; con remoto disponible, ejecución verde en las tres plataformas; si no hay remoto se anota y R-14 queda abierto.

### CP-0 — Punto de control de la preparación
**Corre:** `cargo test-core`; `cargo deny check licenses bans sources`; `cargo build -p dokumentz-app-gpui --release`; `cd apps/tauri && npm ci && npm run tauri build -- --no-bundle`; `cargo test -p dokumentz-sheet --release --test spike_api -- --nocapture`.
**Revisa:** `Cargo.toml` raíz (parches de GPUI, versiones exactas, sin `*`); cabecera de procedencia del código adaptado de `input.rs`; que `research.md` recibió los resultados de T005/T006.
**Decide y pregunta al dueño:** (a) **CG-1** (carga/memoria de IronCalc) y **CG-2** (puente); (b) inicializar git, conectar el remoto `https://github.com/jahziel1/dokumentz` (público, decidido el 2026-10-22) y subir el primer commit, **con confirmación del dueño antes del `push`**; (c) **EX-01** ya aprobada (2026-10-22); (d) si T006 cambia el diseño del núcleo, actualizar `plan.md` antes de seguir.

---

## Fase 1 — Núcleo (fundacional; bloquea lo demás)

### E05 — `dokumentz-metrics` y `dokumentz-script`
- **Agente:** trabajador (~60k). **Lee:** `plan.md` §5.10, §9.7; `spec.md` FR-012. **Toca:** `crates/dokumentz-metrics/**`, `crates/dokumentz-script/**`.

- [x] T012 [P] `dokumentz-metrics`: `Recorder` apagado por defecto (`Recorder::from_args_env()` con `--metrics-out` o `DOKUMENTZ_METRICS_OUT`), marcas, `Span`, `Histogram` (p50/p95/p99 con definición de percentil escrita), contadores, `write_json`; tipos del esquema de resultados de `plan.md` §9.7 en `src/results.rs`. **Verificación:** `cargo test -p dokumentz-metrics` (percentiles contra casos calculados a mano; JSON estable comparado con un archivo dorado).
- [x] T013 [P] `dokumentz-metrics`: pruebas de «apagado» y «sin red»: con el `Recorder` apagado no se crea ningún archivo (directorio temporal vacío) y no se reserva memoria de muestras; `cargo tree -p dokumentz-metrics -e normal` no contiene `reqwest`, `hyper`, `tokio`, `mio`, `socket2`, `ureq`, `curl`, `isahc`. **Verificación:** `cargo test -p dokumentz-metrics --test off_and_no_net` verde.
- [x] T014 [P] `dokumentz-script`: `Script`, `Step` (`open`, `key`, `text`, `scroll`, `drag`, `wait_ms`, `wait_idle`, `mark`, `assert_cell`, `assert_selection`, `assert_formula_bar`, `save`, `quit`), `ScriptResult` (JSON con `serde`) y el binario `dokumentz-script-check <esperado> <resultado>`. **Verificación:** `cargo test -p dokumentz-script`; `cargo run -p dokumentz-script --bin dokumentz-script-check -- tests/fixtures/script_ok.json tests/fixtures/result_ok.json` sale con 0 y con `result_bad.json` sale con 1.

### E06 — Núcleo I: abrir, geometría, lista de dibujo, códec
- **Agente:** trabajador (~110k). **Lee:** `plan.md` §5.1–§5.4 y Anexo A (IronCalc); `crates/dokumentz-sheet/tests/spike_api.rs` (hallazgos). **Toca:** `crates/dokumentz-sheet/src/{lib,core,error,geometry,view,drawlist,codec}.rs`, `crates/dokumentz-sheet/tests/{open_save,geometry,drawlist,codec,view}.rs`, `crates/dokumentz-sheet/tests/golden/drawlist_v1.bin`, `tests/fixtures/{empty,small,corrupt,truncated}.xlsx`.

- [x] T015 `core.rs`, `error.rs`: `Lang`, `OpenOptions`, `Core::new_empty`, `Core::open_xlsx` (`load_from_xlsx` + `UserModel::from_model`, con literales `'static`), errores tipados (`NoExiste`, `SinPermiso`, `NoEsXlsx`); evaluar `load_from_xlsx_bytes` y `save_xlsx_to_writer`; fixtures pequeños generados por un ayudante de pruebas con `rust_xlsxwriter`. **Verificación:** `cargo test -p dokumentz-sheet --test open_save`: abre `small` y `empty`; `corrupt`, `truncated` y una ruta inexistente devuelven el error esperado **sin pánico**.
- [x] T016 `geometry.rs`: `SheetGeometry` (excepciones ordenadas, `col_x/row_y/col_at/row_at`, `extent` = rango usado + una pantalla, constantes `ROW_HEADER_W = 46` y `COL_HEADER_H = 25`). **Verificación:** `cargo test -p dokumentz-sheet --test geometry` con tamaños personalizados y con 1 000 000 filas sintéticas; una prueba `#[ignore]` mide la construcción para 100 000 filas (objetivo ≤ 20 ms en release; si no, usar la lectura de excepciones desde `get_model()` según T006).
- [x] T017 `drawlist.rs`: tipos y `build_block` desde `UserModel` (texto de `get_formatted_cell_value`, estilo deduplicado desde `get_cell_style` con `resolve_color`, alineación `General` resuelta por tipo, `spill_cols`). **Verificación:** `cargo test -p dokumentz-sheet --test drawlist`: el número de celdas del bloque coincide con las celdas con contenido del área pedida; negrita, cursiva, color de texto, relleno y alineación de un fixture aparecen en `CellStyle`; fechas y números salen formateados, no como números de serie.
- [x] T018 `codec.rs`: codificar y decodificar el formato binario v1 de `plan.md` §5.4 y generar `tests/golden/drawlist_v1.bin` desde un fixture determinista. **Verificación:** `cargo test -p dokumentz-sheet --test codec`: ida y vuelta, magia/versión incorrectas, tamaños límite; el dorado se versiona y modificar un byte rompe la prueba (comprobado).
- [x] T019 `view.rs` y `Core::draw_block`/`sheet_metrics`: desplazamiento en píxeles lógicos acotado a la extensión, `SetViewport/ScrollTo/ScrollBy`, caché de bloque por (hoja, rango, `data_version`). **Verificación:** `cargo test -p dokumentz-sheet --test view`: un `draw_block` dentro del bloque cacheado devuelve el mismo `Arc` (`Arc::ptr_eq`); tras un cambio de datos devuelve uno nuevo.

### E07 — Núcleo II: selección y navegación
- **Agente:** trabajador (~80k). **Lee:** `plan.md` §5.5–§5.6; hallazgos de `spike_api.rs`. **Toca:** `crates/dokumentz-sheet/src/{nav,command,view,core}.rs`, `crates/dokumentz-sheet/tests/nav.rs`.

- [x] T020 `Core::hit_test` y los comandos `Click`/`DragTo` (celda, cabecera de fila, cabecera de columna, esquina, fuera). **Verificación:** `cargo test -p dokumentz-sheet --test nav hit_` con columnas de distinto ancho y con desplazamiento.
- [x] T021 `Move`, `Edge`, `Page`, `Home` con `extend` donde la API lo permita (`on_arrow_*`, `on_expand_selected_range`, `on_navigate_to_edge_in_direction`, `on_page_up/down`). **Verificación:** `cargo test -p dokumentz-sheet --test nav`: secuencias de US1-6 (flechas, Ctrl/Cmd+flechas hasta el borde del bloque de datos y de la hoja vacía, Mayús+flechas, RePág/AvPág) dejan la celda activa y el rango esperados en una hoja de 100 000 filas y en una vacía.
- [x] T022 `ensure_visible` y sincronía con `set_top_left_visible_cell`/`set_window_width`/`set_window_height`. **Verificación:** `cargo test -p dokumentz-sheet --test nav visible_`: tras cualquier secuencia aleatoria de movimientos con semilla fija la celda activa está dentro del área visible.

### E08 — Núcleo III: edición, idioma, historial, hojas, portapapeles, guardado
- **Agente:** trabajador (~110k). **Lee:** `plan.md` §5.6–§5.9; hallazgos de `spike_api.rs`; `spec.md` US2 y US3. **Toca:** `crates/dokumentz-sheet/src/{edit,clipboard,save,command,core}.rs` y los tests `edit.rs`, `formulas_es_en.rs`, `undo_deps.rs`, `sheets.rs`, `clipboard.rs`, `save.rs`.

- [x] T023 `edit.rs`: `BeginEdit`/`SetEditText`/`CommitEdit`/`CancelEdit`, regla de avance (Intro baja, Tab a la derecha, Mayús invierte), celdas con texto, fórmula, error y texto largo. **Verificación:** `cargo test -p dokumentz-sheet --test edit` cubre los escenarios 1–3 de US2 (valor guardado, selección baja o pasa a la derecha, Esc conserva el valor anterior).
- [x] T024 Política de idioma de `plan.md` §5.7: implementar la opción (A) (reintento con `undo`) y la (B) (traducción de nombres), medir ambas sobre un A2 sintético, dejar la elegida por el criterio «≤ 2× una edición normal»; `SetLang`. **Verificación:** `cargo test -p dokumentz-sheet --test formulas_es_en`: `SUMA/SUM`, `BUSCARV/VLOOKUP`, `BUSCARX/XLOOKUP`, `SI.ERROR/IFERROR`, `CONTAR.SI/COUNTIF`, `SI/IF` dan el mismo valor en ambos idiomas; una función inexistente deja error y conserva el texto; el historial neto es una entrada; el informe de la etapa incluye los tiempos de (A) y (B).
- [x] T025 Deshacer/rehacer con dependientes. **Verificación:** `cargo test -p dokumentz-sheet --release --test undo_deps`: celda de la que dependen ≥ 10 000 fórmulas; editar → cambian los valores → `Undo` → todos los dependientes vuelven → `Redo` → se reaplican.
- [x] T026 Hojas: `UiState.sheets`, `SelectSheet`; crear, renombrar y borrar solo si cuesta poco (opcional en la PoC). **Verificación:** `cargo test -p dokumentz-sheet --test sheets` con un fixture de 5 hojas (la hoja activa cambia y `draw_block` devuelve el contenido de esa hoja).
- [x] T027 `clipboard.rs`: `copy_text` (TSV con comillas) y `PasteText` (normaliza `\r\n`, recorta tabuladores finales vacíos, usa `paste_csv_string`). **Verificación:** `cargo test -p dokumentz-sheet --test clipboard`: celdas con tabulador, comillas y salto de línea; pegar un 3×3 en medio de la hoja y deshacerlo en un solo paso.
- [x] T028 `save.rs`: guardado atómico (archivo temporal + renombrar), `Save{path}`, marca `dirty`, errores claros (carpeta de solo lectura, disco lleno simulado). **Verificación:** `cargo test -p dokumentz-sheet --test save`: tras un fallo inyectado el archivo original sigue idéntico (hash); guardar y reabrir conserva valores y fórmulas.

### E09 — Núcleo IV: fachada, instrumentación y bancos
- **Agente:** trabajador (~70k). **Lee:** `plan.md` §5.5, §5.10–§5.11, §2.6. **Toca:** `crates/dokumentz-sheet/src/{core,command,lib}.rs`, `crates/dokumentz-sheet/Cargo.toml` (característica `metrics`), `crates/dokumentz-sheet/examples/{bench_drawlist,bench_edit}.rs`, `crates/dokumentz-sheet/tests/{facade,invariants}.rs`.

- [x] T029 `Core::apply`, `ui_state`, serde de `Command` y `UiState` (JSON estable con `data_version`), rustdoc de la API pública con un ejemplo en `Mutex`. **Verificación:** `cargo test -p dokumentz-sheet --doc` y `cargo test -p dokumentz-sheet --test facade` (cada `Command` produce el `UiState` esperado; JSON de ida y vuelta).
- [x] T030 [MIDE] Característica `metrics`: spans en abrir, guardar, evaluar, `draw_block` y codificar; `examples/bench_drawlist.rs` (pantalla + margen sobre un libro de 100 000×20). **Verificación:** `cargo run -p dokumentz-sheet --example bench_drawlist --release --features metrics -- --metrics-out results/dev/T030.json` crea el JSON; p95 de `draw_block` ≤ 3 ms en el Mac (presupuesto de `plan.md` §2.6) o se anota y se optimiza antes de seguir; sin `--metrics-out` no se crea ningún archivo.
- [x] T031 [MIDE] `examples/bench_edit.rs`: `CommitEdit` sobre un A2 sintético (≥ 10 000 dependientes): tiempo total y de `evaluate`, con y sin `pause_evaluation`. **Verificación:** JSON en `results/dev/T031.json`; los números se copian a `research.md` (fila «Recálculo incremental de IronCalc») y deciden entre las opciones (a), (b) y (c) de R-04.
- [x] T032 Invariante FR-003: `crates/dokumentz-sheet/tests/invariants.rs` comprueba que el número de celdas de la lista y el tamaño del bloque dependen del área visible y **no** del tamaño de la hoja (libros de 1 000, 100 000 y 1 000 000 filas con el mismo viewport). **Verificación:** `cargo test -p dokumentz-sheet --test invariants` verde.

### CP-1 — Núcleo completo
**Corre:** `cargo test -p dokumentz-sheet`; `cargo test -p dokumentz-sheet --test no_ui_deps`; `cargo clippy -p dokumentz-sheet -- -D warnings`; `cargo deny check licenses bans sources`; `cargo run -p dokumentz-sheet --example bench_drawlist --release --features metrics -- --metrics-out results/dev/CP1.json`.
**Revisa:** `crates/dokumentz-sheet` en busca de lógica de interfaz (III), código de ONLYOFFICE (VII), API pública documentada, ausencia de `unwrap` en caminos de entrada; el dorado `drawlist_v1.bin` queda **congelado** (un cambio exige v2 y avisar a las dos interfaces).
**Decide:** continuar con las interfaces; anotar R-04 con los números de T031.
**Resultado (2026-10-23):** todo en verde (`cargo test-core`; `dokumentz-sheet` con y sin `metrics`: 256 pruebas; `no_ui_deps`; clippy con `-D warnings`; `cargo deny`; sin `unwrap` en caminos de entrada; sin documentación pública faltante). Dorado `drawlist_v1.bin` congelado. T024 eligió (B), traducción (≈ 1,0× una edición; (A) con pausa ≈ 2,0× y rompe Rehacer). R-04: se adopta (a) y se aplaza (c) (T031: `CommitEdit` ≈ 31,5 ms en A2, lineal en celdas). **T030 en el límite:** p95 de `draw_block` = 2,03 ms con la máquina poco cargada (E09) y 3,06–3,20 ms en CP-1 con carga media ≈ 7,6 de 8 núcleos (otro proceso al 100 %); falta repetirlo con la máquina libre (`plan.md` §9.1) antes de darlo por cumplido o decidir optimizar `build_block`.

---

## Fase 2 — Generador y medición (US4)

### E10 — Generador A1–A3
- **Agente:** trabajador (~70k). **Lee:** `spec.md` §7 (A1–A3) y FR-013; `plan.md` §2.2, §2.3; docs de `rust_xlsxwriter` (`Formula`). **Toca:** `tools/testdata-gen/**`.

- [ ] T033 [P] [US4] `testdata-gen` A1: RNG `rand_chacha` con semilla; 100 000 filas × 20 columnas de números, textos, fechas, porcentajes y moneda; estilos variados (negrita, cursiva, color de texto, relleno, alineación); anchos de columna distintos; fila de títulos. **Verificación:** `cargo test -p testdata-gen` (forma, tipos por columna, estilos presentes) y `cargo run -p testdata-gen --release -- --seed 42 --only A1 --out testdata/` produce `A1.xlsx` en < 60 s.
- [ ] T034 [US4] `testdata-gen` A2: A1 más ≥ 10 000 fórmulas: celda de entrada `U1`, dependientes en `V2:V10001` y `W2:W10001` (mezcla de `=$U$1*Tn+Sn`, cadenas cortas, `SI`, `BUSCARV`/`BUSCARX`, `SUMA`) escritas con nombres en inglés y con resultado en caché correcto (`Formula::new(..).set_result(..)`). **Verificación:** una prueba recalcula esas fórmulas con aritmética propia y compara con el caché; `A2.xlsx` abre en `dokumentz-sheet` y `evaluate()` no cambia ningún valor.
- [ ] T035 [US4] `testdata-gen` A3 (5 hojas × 10 000 filas), `empty.xlsx`, `--scale ci` (A1–A3 reducidos a 2 000 filas, escritos en `<out>/ci/`), CLI completa (`--seed`, `--only`, `--all`, `--scale`, `--out`) y `manifest.json` (sha256 del contenido, forma, semilla, versión). **Verificación:** `cargo run -p testdata-gen --release -- --seed 42 --all --out testdata/` y con `--scale ci`; `manifest.json` rellenado; `cargo test -p testdata-gen` verde.

### E11 — Comparador, juez y reproducibilidad
- **Agente:** trabajador (~70k). **Lee:** `plan.md` §2.4, §9.3 (M-07); `spec.md` FR-011, SC-005. **Toca:** `tools/xlsx-diff/**`, `tools/bench-runner/src/judge.rs`, `tools/testdata-gen/tests/reproducible.rs`.

- [ ] T036 [P] [US4] `xlsx-diff` (con `calamine`, lector independiente de IronCalc): compara valores y fórmulas hoja por hoja y celda por celda (tolerancia numérica 1e-9; fechas como números de serie), salida legible y código de salida 0/1; biblioteca y binario. **Verificación:** `cargo test -p xlsx-diff` con pares iguales y con diferencias de valor, de fórmula y de hoja faltante.
- [ ] T037 [P] [US4] `bench-runner judge --x2t <ruta> --file <xlsx>`: ejecuta `x2t <in.xlsx> <out.xlsx>` y `x2t <in.xlsx> <out.csv>` (la propia herramienta calcula la dirección por la extensión), exige código 0 y salida no vacía y compara `in` con `out` con `xlsx-diff` en modo «solo valores»; opción `--soffice <ruta>` para LibreOffice headless. Solo se **invoca** el programa (principio VII). **Verificación:** en el Mac, `cargo run -p bench-runner -- judge --x2t "/Applications/ONLYOFFICE.app/Contents/Resources/converter/x2t" --file tests/fixtures/small.xlsx` sale con 0 y escribe un JSON; con `corrupt.xlsx` sale distinto de 0.
- [ ] T038 [US4] Reproducibilidad de FR-013: genera A1–A3 reducidos (`--scale ci`) dos veces con la misma semilla en directorios distintos y compara con `xlsx-diff`; con otra semilla debe haber diferencias. **Verificación:** `cargo test -p testdata-gen --test reproducible`; además dos ejecuciones de `testdata-gen --seed 42 --all` producen `manifest.json` idénticos.

### E12 — `bench-runner`: esquema, entorno, memoria
- **Agente:** trabajador (~100k). **Lee:** `plan.md` §9.1, §9.4–§9.7. **Toca:** `crates/dokumentz-metrics/src/results.rs`, `tools/bench-report/schema/results.v1.schema.json`, `tools/bench-runner/src/{main,env,preflight,mem,procs}.rs`, `tools/bench-runner/apps.toml`.

- [ ] T039 [US4] Esquema de resultados: completar los tipos de `dokumentz-metrics::results` y publicar el JSON Schema `tools/bench-report/schema/results.v1.schema.json`. **Verificación:** `cargo test -p dokumentz-metrics results` y `cargo test -p bench-report schema` validan el ejemplo de `plan.md` §9.7 y rechazan uno con un campo requerido ausente.
- [ ] T040 [US4] `bench-runner env` y `preflight`: hardware, SO, GPU, pantalla (resolución, escala, frecuencia), VM sí/no, corriente, versión del motor web (WKWebView por versión de macOS; WebView2 por registro; WebKitGTK por `pkg-config`); comprueba `plan.md` §9.1-8. **Verificación:** en el Mac, `cargo run -p bench-runner -- env` imprime el JSON del entorno (`refresh_hz` no nulo, o `null` con aviso si el SO no lo da); `preflight` falla de forma legible con una aplicación pesada abierta.
- [ ] T041 [P] [US4] Muestreo de memoria por árbol de procesos (`mem.rs`, `procs.rs`): macOS (`footprint -a -j` + diferencia de PIDs `com.apple.WebKit.*`), Windows (conjunto de trabajo privado), Linux (`Pss` de `smaps_rollup`), con `cfg(target_os)`. **Verificación:** `cargo test -p bench-runner mem`: un proceso de prueba que lanza hijos y reserva memoria conocida se suma completo; la CI lo ejecuta en los tres SO.

### E13 — `bench-runner`: captura, entrada, análisis
- **Agente:** trabajador (~120k). **Lee:** `plan.md` §9.3, §9.4, §9.9. **Toca:** `tools/bench-runner/src/{capture,input,analyze,calibrate,scenario}.rs` y sus pruebas.

- [ ] T042 [US4] Captura con `ffmpeg` por SO (`capture.rs`): construye la línea de comandos de `plan.md` §9.4, lanza el proceso, lee cuadros en gris de la región de interés (`-f rawvideo -pix_fmt gray` por tubería) con marca de tiempo de reloj de pared y detecta huecos. **Verificación:** prueba con vídeo sintético (`ffmpeg -f lavfi -i testsrc`) que entrega N cuadros con tiempos crecientes; en el Mac se anota la tasa máxima real de `avfoundation` pidiendo 60 y 120 cuadros por segundo; si no alcanza el doble de la frecuencia de refresco, se abre una tarea de ayudante en Swift (plan B de `plan.md` §9.4).
- [ ] T043 [P] [US4] Entrada sintética (`input.rs`): trait `InputDriver` (`key`, `text`, `wheel`, `drag`, `place_window`) con implementaciones `cfg`: macOS (`CGEvent`), Windows (`SendInput`), Linux X11 (XTest) y un modo `--dry-run` que solo registra. **Verificación:** compila en los tres SO (CI); en el Mac, con TextEdit abierto, `bench-runner input --demo` escribe un texto y mueve la rueda (requiere los permisos del sistema, ver T084).
- [ ] T044 [US4] Analizador de cuadros (`analyze.rs`, funciones puras sobre `Vec<Frame>`): detección de cambios por región, estado estable (≥ 300 ms), primer cuadro igual al final, intervalos entre cambios, cuadros perdidos, latencia por tecla con umbral de píxeles frente al parpadeo del cursor. **Verificación:** `cargo test -p bench-runner analyze` con secuencias sintéticas de resultado conocido (intervalos de 16,7 ms con tres pérdidas; teclas con retardo fijo de 20 ms).
- [ ] T045 [US4] `calibrate` y `scenario`: `calibrate --target <exe>` mide el «piso del método» sobre una ventana que cambia de color al recibir una tecla y escribe `floor_ms`; `scenario` ejecuta M-01…M-07 con el procedimiento de `plan.md` §9.3 y §9.6 y escribe el JSON de §9.7. **Verificación:** `cargo test -p bench-runner scenario` con `InputDriver` y captura simulados; `bench-runner run --help` documenta las opciones. (La ejecución real ocurre en E27/E28.)

### E14 — `bench-report`: agregación, criterios, decisión
- **Agente:** trabajador (~90k). **Lee:** `spec.md` §6, §8, §9; `plan.md` §9.7–§9.8. **Toca:** `tools/bench-report/**`.

- [ ] T046 [US4] Agregación (`spec.md` §6): p95 por repetición y mediana entre repeticiones más el peor (M-03, M-04); mediana, p95, mínimo y máximo del resto; frío y caliente por separado; repeticiones inválidas contadas. **Verificación:** `cargo test -p bench-report aggregate` con JSON sintético y resultados calculados a mano.
- [ ] T047 [US4] Criterios en `tools/bench-report/criteria.toml` y `criteria.rs`: SC-001 (≥ 2× en M-02; ≤ 50 % en M-06), SC-002 (16,7 ms; ideal 8,3 ms), SC-003 (30 ms), SC-004 (entrada manual), SC-005 (`xlsx-diff` sin diferencias), SC-006 (CI), SC-007 ((a) ≥ 95 % de archivos sin error y 0 cierres; (b) ≥ 98 % de fórmulas coincidentes; (c) 100 % de valores sin fórmula idénticos; umbral de 90 % de la regla del motor; tolerancia numérica relativa 1e-9; la evaluación se implementa en T117, pero los umbrales se congelan aquí, antes de cualquier corrida oficial); estados `cumple` / `no cumple` / `sin datos`. **Verificación:** `cargo test -p bench-report criteria` con casos de frontera (exactamente 2,0×, 50 %, 16,7 ms) y dato ausente → `sin datos`, nunca `cumple`.
- [ ] T048 [US4] Regla de decisión de `spec.md` §9 (orden 1→4 más el caso «no cubierto», con la lectura de las aclaraciones 9 y 10) y salida `informe.md` + `informe.json` (`plan.md` §9.8). **Verificación:** `cargo test -p bench-report decision` con un caso por rama (ninguna supera a ONLYOFFICE; Tauri cumple y GPUI es ≥ 2× mejor; Tauri cumple; Tauri falla y GPUI cumple; no cubierto) y una prueba de instantánea del Markdown.
- [ ] T049 [US4] [BLOQUEADA: requiere git (CP-0)] Congelar los criterios (umbrales adoptados por defecto, aclaración 11 de `spec.md`): confirmar `criteria.toml`, crear la etiqueta `criterios-v1` (requiere git) y hacer que `bench-runner` y las PoC escriban `criteria_sha` en cada JSON. **Verificación:** `git tag -l criterios-v1` existe; un JSON de prueba contiene el hash. Sin esta tarea no hay corrida oficial (principio VIII).

### CP-2 — Generador y medición listos
**Corre:** `cargo test-core`; `cargo run -p testdata-gen --release -- --seed 42 --all --out testdata/` (dos veces, `manifest.json` idéntico); `cargo test -p bench-report`; `cargo run -p bench-runner -- judge … small.xlsx`.
**Revisa:** que `bench-report` implementa **exactamente** la tabla de `spec.md` §9; `criteria.toml` contra `spec.md` §8; que ninguna herramienta lee ni copia archivos de ONLYOFFICE más allá de ejecutar `x2t`.
**Decide:** continuar con las interfaces; recordar al dueño que T049 debe estar hecha antes de E28.


---

## Fase 3 — US1: abrir y recorrer una hoja grande (P1) — FR-001…FR-004; M-01, M-02, M-03

Las etapas de Tauri (E15–E17) y de GPUI (E18–E19) **son paralelas**: tocan directorios distintos. E20 espera a ambas.

### E15 — Tauri: backend
- **Agente:** trabajador (~80k). **Lee:** `plan.md` §6.1–§6.2, §6.8, §5.10; `crates/dokumentz-sheet/src/{lib,command}.rs` (API pública). **Toca:** `apps/tauri/src-tauri/**`.

- [ ] T050 [US1] `dokumentz-app-tauri`: `state.rs` (`Mutex<Core>`), `cli.rs` (`--open` y ruta posicional, `--lang`, `--metrics-out`, `--script`, `--script-out`, `--exit-after-script`), `commands.rs` con comandos `async` (`open_xlsx`, `draw_block` → `tauri::ipc::Response`, `exec`, `copy_text`, `metrics_flush`), canal `events`, apertura y operaciones largas en `spawn_blocking`; un `--open` con error termina con código ≠ 0. **Verificación:** `cargo test -p dokumentz-app-tauri`: las funciones de `commands` sobre un `Core` real (abrir `small.xlsx`, `draw_block` decodificable con el decodificador de Rust del dorado, `exec` mueve la selección); `cargo build -p dokumentz-app-tauri --release`.
- [ ] T051 [US1] Seguridad y red: `tauri.conf.json` con CSP `default-src 'self'` sin orígenes remotos y `capabilities/default.json` con los permisos mínimos de `plan.md` §6.8; ninguna URL `http(s)://` en el frontend ni en la configuración. **Verificación:** `cargo test -p dokumentz-app-tauri --test no_remote` busca `https?://` en `apps/tauri/src/`, `tauri.conf.json` y `capabilities/` fuera de una lista permitida (`ipc.localhost`, `tauri.localhost`); la ausencia del aviso «IPC custom protocol failed» en consola se comprueba en T056.

### E16 — Tauri: frontend I (decodificador, canvas, desplazamiento)
- **Agente:** trabajador (~100k). **Lee:** `plan.md` §5.4, §6.3–§6.5; `crates/dokumentz-sheet/tests/golden/drawlist_v1.bin`; los spikes `apps/tauri/src/spike*.ts`. **Toca:** `apps/tauri/src/{drawlist,renderer,scroller}.ts` y sus `*.test.ts`, `apps/tauri/package.json` (scripts de prueba).

- [ ] T052 [US1] `drawlist.ts`: decodificador del formato v1 (`DataView`) con pruebas Vitest contra `crates/dokumentz-sheet/tests/golden/drawlist_v1.bin` (mismo contenido que verifica Rust: celdas, estilos, aristas, etiquetas). **Verificación:** `cd apps/tauri && npm test -- drawlist` verde; cambiar un byte del dorado rompe la prueba.
- [ ] T053 [US1] [MIDE] `renderer.ts`: canvas 2D de `plan.md` §6.5 (fondos agrupados, cuadrícula de 1 px físico, cabeceras, texto con caché de `measureText`, recorte solo si hace falta, `spill_cols`, `####`, selección y celda activa en `#overlay`, DPR); las decisiones de texto son funciones puras con pruebas. **Verificación:** `npm test -- renderer`; el contador `cells_drawn_per_frame` queda expuesto; captura de `testdata/ci/A1.xlsx` en el Mac guardada en `results/dev/` y comparada a ojo con ONLYOFFICE (FR-002).
- [ ] T054 [US1] [MIDE] `scroller.ts`: espaciador escalado (`MAX_DOM_PX`), caché de 3 bloques, precarga predictiva, descarte por generación, `placeholder_frames`. **Verificación:** `npm test -- scroller` con un `invoke` simulado de 2–20 ms: 10 s de rueda simulada → 0 cuadros marcador; un salto de barra los produce y se resuelve; extensión de 1 000 000 filas con `scaleY > 1` → la posición de la barra corresponde a la fila mostrada.

### E17 — Tauri: frontend II (entrada, barra de fórmulas, pestañas, guion)
- **Agente:** trabajador (~100k). **Lee:** `plan.md` §6.6–§6.9, §5.10; API de `crates/dokumentz-script`. **Toca:** `apps/tauri/src/{input,formulabar,tabs,main,script,metrics}.ts`, `apps/tauri/index.html`, estilos.

- [ ] T055 [US1] `input.ts` (teclado y ratón de FR-004 con la tabla de atajos de §6.7; Cmd en macOS, Ctrl en el resto), `formulabar.ts` (solo lectura en US1), `tabs.ts` (muestra las hojas), `main.ts` (une todo y abre el diálogo con `tauri-plugin-dialog`). **Verificación:** `npm test` (mapeo de teclas por SO con eventos sintéticos: Cmd+↓ en macOS y Ctrl+↓ en los demás → `Edge`); `npm run tauri build -- --no-bundle` y `dokumentz-app-tauri --open testdata/ci/A1.xlsx` muestra la cuadrícula (captura en `results/dev/`).
- [ ] T056 [US1] [MIDE] `metrics.ts` y `script.ts`: instrumentación interna de `plan.md` §6.9 y ejecutor de guion que invoca los mismos manejadores de `input.ts`; `metrics_flush` escribe el JSON. **Verificación:** `dokumentz-app-tauri --open testdata/A1.xlsx --script tests/functional/us1/scroll.json --metrics-out results/dev/T056.json --exit-after-script` genera un JSON válido contra el esquema de T039 con `cells_drawn_per_frame` y M-02 interno; sin `--metrics-out` no se escribe nada; la consola no muestra caída a `postMessage`.

### E18 — GPUI: cuadrícula
- **Agente:** trabajador (~110k). **Lee:** `plan.md` §7.1–§7.4, §7.7; el spike `apps/gpui/src/main.rs`; `crates/dokumentz-sheet` (API pública). **Toca:** `apps/gpui/src/{main,app,cli,grid,scrollbar}.rs`.

- [ ] T057 [US1] Shell: `main.rs`, `app.rs` (entidad `AppState` con `Arc<Mutex<Core>>`, ventana de 1280×720), `cli.rs` (mismas banderas que Tauri **más** `--calibrate`, que abre una ventana que cambia de color con cualquier tecla, para T045/T084), `prompt_for_paths` para abrir, familia de fuente por SO. **Verificación:** `cargo build -p dokumentz-app-gpui --release`; `cargo run -p dokumentz-app-gpui --release -- --open testdata/ci/A1.xlsx` abre sin error (captura en `results/dev/`); `--open` con un archivo corrupto sale con código ≠ 0.
- [ ] T058 [US1] [MIDE] `grid.rs`: `GridElement` (`plan.md` §7.2): `draw_block`, caché de `ShapedLine` por (texto, estilo), `with_content_mask`, fondos, líneas, cabeceras, selección y celda activa, `####` y desbordamiento (EX-01). **Verificación:** `cargo test -p dokumentz-app-gpui grid_` (lógica pura de decisión de texto y recortes); con la instrumentación, `prepaint` + `paint` por cuadro con p95 ≤ 6 ms en el Mac (si no, optimizar la caché antes de seguir); captura comparada con la de Tauri.
- [ ] T059 [US1] `scrollbar.rs` y rueda/trackpad (`on_scroll_wheel`, `pixel_delta`): barras de 14 px siempre visibles con arrastre y posición proporcional a la extensión. **Verificación:** `cargo test -p dokumentz-app-gpui scroll_` (mapeo píxel↔extensión y arrastre); a mano en A1: arrastrar la barra hasta el final muestra la última fila con datos y las cabeceras coinciden (US1-5).

### E19 — GPUI: entrada, barra de fórmulas, pestañas, guion
- **Agente:** trabajador (~90k). **Lee:** `plan.md` §7.5–§7.8, §5.10; Zed `examples/input.rs` (referencia de acciones y teclas). **Toca:** `apps/gpui/src/{keymap,formula_bar,tabs,script,metrics}.rs`, `apps/gpui/src/grid.rs`.

- [ ] T060 [US1] `keymap.rs` (dos tablas con `cfg!(target_os = "macos")` según `plan.md` §7.6), acciones `Move`/`Edge`/`Page`, clic y arrastre (`Click`, `DragTo`), `formula_bar.rs` (solo lectura en US1), `tabs.rs` (muestra las hojas). **Verificación:** `cargo test -p dokumentz-app-gpui keymap_` (cada atajo → la acción esperada por SO); con `TestAppContext`, una secuencia de teclas sobre un `Core` pequeño deja la selección esperada.
- [ ] T061 [US1] [MIDE] `metrics.rs` y `script.rs`: instrumentación de `plan.md` §7.8 (incluye `refresh_interval`) y ejecutor de guion con los mismos manejadores. **Verificación:** `dokumentz-app-gpui --open testdata/A1.xlsx --script tests/functional/us1/scroll.json --metrics-out results/dev/T061.json --exit-after-script` genera un JSON válido contra el esquema; sin la bandera no escribe nada.

### E20 — Pruebas y medición interna de US1
- **Agente:** trabajador para T062, mecanico para ejecutar T063 (~70k). **Lee:** `spec.md` US1 (escenarios 1–7); `plan.md` §8. **Toca:** `tests/functional/us1/*.json`, `tests/fixtures/*`, `results/macos-m1/dev/US1/`.

- [ ] T062 [US1] Guiones `tests/functional/us1/{open,nav,scroll,edge}.json` con `assert_cell`, `assert_selection` y `assert_formula_bar` sobre `testdata/ci/A1.xlsx` y `empty.xlsx`: flechas, Cmd/Ctrl+flechas hasta el borde, Mayús+flechas, RePág/AvPág, clic y arrastre, desplazamiento hasta el final con cabeceras sincronizadas, hoja vacía. **Verificación:** para cada app, `<app> --script tests/functional/us1/nav.json --exit-after-script --script-out out.json && dokumentz-script-check tests/functional/us1/nav.expected.json out.json` sale con 0 en **ambas**, y los resultados de ambas apps son idénticos.
- [ ] T063 [US1] [MIDE] Medición interna en el Mac con A1: M-01, M-02 y M-03 internas de ambas apps (5 repeticiones, `--metrics-out`) y `cells_drawn_per_frame` (FR-003). **Verificación:** JSON en `results/macos-m1/dev/US1/` válidos contra el esquema; las celdas dibujadas por cuadro son ≈ constantes entre `testdata/ci/A1.xlsx` y A1; el resumen de números va en el informe de la etapa (no es la corrida oficial y no evalúa SC).

### CP-3 — US1 en las dos interfaces
**Corre:** `cargo test-core`; `cargo test -p dokumentz-app-tauri`; `cargo test -p dokumentz-app-gpui`; `cd apps/tauri && npm test`; los guiones de T062 en **ambas** apps con `dokumentz-script-check`; `cargo deny check`.
**Revisa:** que ninguna interfaz formatea números, resuelve celdas por coordenadas ni lee archivos (III); `scroller.ts` no llama a `invoke` en el camino de pintado; que `grid.rs` no crea un elemento por celda; paridad funcional (`plan.md` §8, filas FR-001…FR-004).
**Decide:** continuar con US2; si T058 o T054 incumplen sus presupuestos internos, se anota el hallazgo y se decide si se optimiza ahora o se deja a la medición oficial.

---

## Fase 4 — US2: editar y recalcular, en español (P1) — FR-005…FR-008; M-04, M-05

### E21 — Tauri: edición e IME
- **Agente:** trabajador (~110k). **Lee:** `plan.md` §6.6, §5.6–§5.7; `spec.md` US2 y casos límite de IME. **Toca:** `apps/tauri/src/{editor,ime,formulabar,input}.ts` y sus pruebas, `index.html` y estilos.

- [ ] T064 [US2] `editor.ts`: textarea oculto; entrada en edición al teclear, con F2, con Ctrl+U (macOS), con doble clic o con clic en la barra de fórmulas; texto de semilla; reflejo en la barra de fórmulas; Intro/Tab/Mayús; Esc; cola de teclas mientras `busy`. **Verificación:** `npm test -- editor` (secuencias sintéticas → llamadas esperadas a `exec`); escenarios US2-1/2/3 con el guion `tests/functional/us2/edit.json` (T070).
- [ ] T065 [US2] Barra de fórmulas editable, atajos de deshacer/rehacer por SO, visualización de errores (`#DIV/0!`, `#REF!`…) y de texto largo (la barra muestra el contenido completo). **Verificación:** `npm test`; guion `tests/functional/us2/undo.json` (T070).
- [ ] T066 [US2] `ime.ts` (máquina de estados Idle/Composing/JustEnded) y los casos límite de `spec.md` §4: Intro durante la composición, clic en otra celda durante la composición, composición con desplazamiento (anclaje al borde visible), tecla muerta seguida de un carácter incompatible. **Verificación:** `npm test -- ime` con secuencias de eventos de composición sintéticas para cada caso; la prueba real con un IME se hace en T072.

### E22 — GPUI: edición e IME
- **Agente:** trabajador (~120k). **Lee:** `plan.md` §7.5; Zed `examples/input.rs`; `spec.md` US2. **Toca:** `apps/gpui/src/{text_input,editor,formula_bar,keymap,grid}.rs`.

- [ ] T067 [US2] `text_input.rs`: `TextInput` reutilizable adaptado de `input.rs` de Zed (cabecera con procedencia y licencia Apache-2.0) con `EntityInputHandler` completo, Inicio/Fin, selección con Mayús, copiar/pegar dentro del editor y búfer de edición compartido con la celda. **Verificación:** `cargo test -p dokumentz-app-gpui text_input_` con `TestAppContext`: `replace_and_mark_text_in_range` + `replace_text_in_range` simulan una composición; rangos UTF-16 correctos con `ñ`, `é` y caracteres de dos unidades UTF-16; `cargo deny check` sigue verde.
- [ ] T068 [US2] `editor.rs`: editor en la celda (`handle_input`, candidatos del IME anclados con `bounds_for_range`), entrada en edición por teclear / F2 / Ctrl+U / doble clic / clic en la barra, sincronía con la barra de fórmulas, Intro/Tab/Esc, deshacer/rehacer, cola de teclas mientras `busy`. **Verificación:** `cargo test -p dokumentz-app-gpui editor_`; el guion `tests/functional/us2/edit.json` pasa.
- [ ] T069 [US2] Casos límite del IME en GPUI (los mismos de T066): composición que sobrevive al desplazamiento, Intro durante la composición, clic en otra celda durante la composición. **Verificación:** `cargo test -p dokumentz-app-gpui ime_` con `TestAppContext`; los casos reales se prueban en T072.

### E23 — Pruebas, medición interna y guion manual de US2
- **Agente:** trabajador para T070 y T072 (documento), mecanico para ejecutar T071, persona para la pasada manual (~80k). **Lee:** `spec.md` US2; `plan.md` §9.3 (M-04, M-05), §6.2 (decisión del `Channel`). **Toca:** `tests/functional/us2/*.json`, `tests/manual/fr007-ime.md`, `results/macos-m1/dev/US2/`; las apps solo en lo que pida T071.

- [ ] T070 [US2] Guiones `tests/functional/us2/{edit,formulas_es_en,undo,deps}.json`: Intro/Tab, F2, Esc, barra de fórmulas, la lista fija de fórmulas es/en, error de nombre, `testdata/ci/A2.xlsx` abierto en `es` con nombres de función en español, cambio de `U1` que actualiza dependientes, deshacer/rehacer con ≥ 10 000 dependientes. **Verificación:** ambas apps pasan con `dokumentz-script-check` y sus resultados coinciden con los del núcleo (T024, T025).
- [ ] T071 [US2] [MIDE] Medición interna de M-04 (tecla → cuadro) y M-05 (Intro → dependientes visibles) en ambas apps con A2 en el Mac, 5 repeticiones; con los números, decidir si hace falta empujar el bloque por `Channel` tras `CommitEdit` en Tauri y si la opción (a) o (b) de R-04 es necesaria. **Verificación:** JSON en `results/macos-m1/dev/US2/`; resumen y decisión en el informe de la etapa.
- [ ] T072 [US2] [MANUAL] Guion `tests/manual/fr007-ime.md` (tabla plataforma × app: á, ñ, ü con tecla muerta; japonés con IME; chino pinyin; dictado en macOS; los casos límite de IME de `spec.md` §4; resultado sí/no/parcial y observaciones) y primera pasada en macOS por una persona con las dos apps. **Verificación:** tabla de macOS rellena; cada fallo se vuelve una tarea nueva o se anota en R-07.

### CP-4 — US2 en las dos interfaces
**Corre:** `cargo test-core`; `cargo test -p dokumentz-app-tauri -p dokumentz-app-gpui`; `npm test`; los guiones `us1` y `us2` en ambas apps con `dokumentz-script-check`.
**Revisa:** que el texto en edición vive solo en la interfaz hasta `CommitEdit` y que ninguna interfaz interpreta fórmulas ni nombres de función (III); procedencia de `text_input.rs`; resultados de T071 contra los presupuestos de `plan.md` §2.6.
**Decide:** continuar con US3; abrir tareas por cada fallo de T072.

---

## Fase 5 — US3: varias hojas, copiar/pegar y guardar sin pérdidas (P2) — FR-009…FR-011; M-07 — y US5: compatibilidad del motor (P1) — FR-014; M-09; SC-007 (E33)

### E24 — Hojas, portapapeles y guardado en las dos interfaces
- **Agente:** trabajador (~110k). **Lee:** `plan.md` §5.8–§5.9, §6.8, §7.7; `spec.md` US3. **Toca:** `apps/tauri/src/{tabs,input,editor}.ts`, `apps/tauri/src-tauri/src/commands.rs`, `apps/gpui/src/{tabs,keymap,app}.rs`.

- [ ] T073 [US3] Pestañas de hojas interactivas (clic, pestaña activa marcada, A3 con 5 hojas) en ambas apps. **Verificación:** guion `tests/functional/us3/sheets.json` pasa en ambas (`assert_cell` en cada hoja).
- [ ] T074 [US3] Tauri: copiar (`copy_text` → `writeText`) y pegar (`readText` → `PasteText`) con `tauri-plugin-clipboard-manager` y permisos mínimos. **Verificación:** `npm test` (con el plugin simulado); manual en el Mac: copiar A1:C3 de la app y pegar en ONLYOFFICE llega tabulado; copiar de ONLYOFFICE y pegar en la app rellena hacia abajo y a la derecha.
- [ ] T075 [US3] GPUI: copiar/pegar con `write_to_clipboard(ClipboardItem::new_string(..))` y `read_from_clipboard()`. **Verificación:** `cargo test -p dokumentz-app-gpui clipboard_` (si `TestAppContext` no simula el portapapeles `[NO VERIFICADO]`, prueba manual como en T074).
- [ ] T076 [US3] Guardar (Cmd/Ctrl+S sobre el archivo abierto) y Guardar como (`dialog.save()` / `prompt_for_new_path`), ejecución en hilo de trabajo, mensajes de error y marca `dirty` en el título. **Verificación:** guion `tests/functional/us3/save.json` (editar → guardar → reabrir → `assert_cell`); guardar en una carpeta de solo lectura muestra un error y el original no cambia (hash).

### E25 — Integridad y jueces
- **Agente:** trabajador para T077, T080 y T079; mecanico para ejecutar T078 (~80k). **Lee:** `spec.md` US3 y SC-005; `plan.md` §2.4. **Toca:** `crates/dokumentz-sheet/tests/roundtrip.rs`, `tools/bench-runner/src/{judge,roundtrip}.rs`, `tests/functional/us3/*.json`, `results/macos-m1/dev/US3/`.

- [ ] T077 [US3] Ida y vuelta automática (SC-005): `crates/dokumentz-sheet/tests/roundtrip.rs` (versión `--scale ci`, corre en CI) y `bench-runner roundtrip --dir testdata/` (A1–A3 completos, local): abrir → guardar → reabrir → `xlsx-diff` contra el original. **Verificación:** `cargo test -p dokumentz-sheet --test roundtrip` verde; `cargo run -p bench-runner --release -- roundtrip --dir testdata/` informa 0 diferencias para A1, A2 y A3 (valores y fórmulas) o la lista exacta de las diferencias.
- [ ] T078 [US3] Jueces (principio VII): `bench-runner judge` con `x2t` sobre los `.xlsx` guardados por **cada** app y por el núcleo; apertura manual en ONLYOFFICE (sin avisos de reparación), en LibreOffice si está instalado y en Excel si está disponible, con la versión de cada juez registrada. **Verificación:** tabla juez × archivo en `results/macos-m1/dev/US3/jueces.json` y capturas revisadas; cualquier aviso de reparación cuenta como fallo.
- [ ] T079 [US3] [MIDE] M-07 interno de ambas apps (guardar A1 tras editar una celda) y comprobación de integridad posterior con `xlsx-diff`. **Verificación:** JSON en `results/macos-m1/dev/US3/`; una repetición con diferencias cuenta como fallo, no como dato.
- [ ] T080 [US3] Guiones `tests/functional/us3/{clipboard,save,sheets}.json` completos y ejecutados en ambas apps, incluidos los casos límite de `spec.md` §4 (comillas, saltos de línea dentro de una celda, tabuladores finales). **Verificación:** ambas pasan `dokumentz-script-check` con resultados idénticos entre apps.
- [ ] T114 [US3] [US5] [MANUAL] Archivos A4 (aclaración 12 de `spec.md`). **Ya no es solo informativa: alimenta SC-007** (T115–T118) y se informa por fuente. `testdata/a4/manifest.toml` (versionado) con, por archivo, `id`, **`fuente` (`usuario` | `apache-poi` | `libreoffice`)**, licencia, tamaño y SHA-256; `tools/corpus/fetch.sh` descarga **solo** los archivos listados de Apache POI (`test-data/spreadsheet`, Apache-2.0, incluidos los 14 `.xlsm`) y de LibreOffice (`sc/qa/unit/data/xlsx`, MPL-2.0) a `testdata/a4/publicos/` y comprueba los hashes; los archivos del usuario se copian a `testdata/a4/usuario/`. **Antes de descargar**, el agente principal pide al dueño el visto bueno con la lista, la fuente y el tamaño total. Los archivos se tratan como no confiables: ninguna macro se ejecuta y no se abren en Excel con macros habilitadas. Después, `bench-runner roundtrip --dir testdata/a4/` y apertura en ambas apps. **Verificación:** `git status` no muestra nada bajo `testdata/a4/` salvo el manifiesto; todo archivo del manifiesto tiene `fuente` válida y su hash coincide; informe `results/macos-m1/dev/US3/a4.json` con abre / falla / diferencias por archivo y el aviso de «macros no conservadas» comprobado en cada `.xlsm`; no decide SC-005 (lo deciden A1–A3) pero es la entrada de T116 (SC-007).

### E33 — Compatibilidad del motor (SC-007)
- **Agente:** trabajador (T115, T117, T118; ~90k) y persona (T116, `[MANUAL]`; un `mecanico` puede ejecutar las órdenes de T116 una vez preparado A4). **Lee:** `spec.md` US5, FR-014, M-09, SC-007 y §9 (regla del motor); `plan.md` §2.2, §4, §5.9, §9.10 y R-18; `research.md` D-17; `tools/xlsx-diff/src` (reutiliza su comparación de valores si encaja). **Toca:** `crates/dokumentz-sheet/src/inspect.rs` (solo lectura) y su prueba, `tools/compat-check/**`, `tests/fixtures/compat/**`, `tools/bench-report/{src,schema}/**` (solo T117), `crates/dokumentz-metrics/src/results.rs` (solo T117), `results/macos-m1/` (T116). No toca las interfaces. Depende del núcleo (CP-1), de `calamine` fijado (T001, T036), de los umbrales congelados (T047, T049) y de A4 preparado (T114, con el visto bueno del dueño).

- [ ] T115 [US5] `tools/compat-check` (biblioteca y binario) según `plan.md` §9.10: (1) en el núcleo, `crates/dokumentz-sheet/src/inspect.rs` de solo lectura (`sheet_names`, `used_range`, `cell_raw`, `cell_formula`, `recalculate`; `plan.md` §5.9); (2) supervisor que lanza **un proceso hijo por archivo** con tiempo máximo y distingue error de apertura, tiempo agotado y cierre inesperado, con subcomandos `run --manifest <toml> --dir testdata/a4/ --out <carpeta>`, `file <xlsx>` (hijo, JSON por salida estándar) y `fixtures <carpeta> --out <carpeta>`; (3) detección del productor por `docProps/app.xml` (`Application`) y registro de `calcPr`; (4) referencia independiente con `calamine` 0.37.0 (valores guardados y fórmulas; sin llamar a `vba_project`); (5) recálculo con `dokumentz-sheet` en idioma `en` y zona `UTC`; (6) comparación con las tolerancias de §9.10 (números con tolerancia relativa 1e-9; texto y booleanos exactos; errores del mismo tipo; fechas como número de serie); (7) exclusión de volátiles (`TODAY`, `NOW`, `RAND`, `RANDBETWEEN`, `RANDARRAY`, `CELL`, `INFO`) y de sus dependientes, contados aparte; (8) celdas sin valor guardado contadas aparte; (9) comparación de los valores sin fórmula; (10) lista de funciones no soportadas (`#NAME?`) o con resultados distintos, por frecuencia; (11) salida `M-09-compat.json` (esquema `dokumentz-bench/1`, `method: "compat"`) y `compat.md`, con el JSON versionado sin valores de celda, hojas ni rutas de la fuente `usuario` (el detalle va a `privado/`). **Nunca ejecuta macros.** Resuelve los `[A VERIFICAR EN T115]` de `plan.md`. Fixtures en `tests/fixtures/compat/` y en las pruebas, generados con `rust_xlsxwriter` + `set_formula_result` (y reescribiendo el ZIP cuando haga falta quitar un `<v>` o cambiar `app.xml`). **Verificación:** `cargo test -p compat-check` verde, con una prueba por caso: (i) valor guardado igual al recalculado → coincide; (ii) valor guardado distinto del real (`=A1+B1` guardado como 7 con real 5) → discrepancia, lo que prueba que se compara contra el valor guardado y que el valor del motor viene del recálculo; (iii) error igual y error distinto; (iv) tipo distinto (número contra texto); (v) `TODAY()`/`NOW()`/`RAND()` con valor guardado arbitrario y una celda que depende de ellas → excluidas y contadas aparte; (vi) fórmula sin `<v>` → `no_stored_value`; (vii) productor `Microsoft Excel` entra en (b), `LibreOffice`, `Apache POI` y sin `app.xml` se informan aparte; (viii) función que el motor no reconoce → aparece en la lista (d); (ix) valores sin fórmula (número, texto, booleano, fecha) idénticos → (c) = 100 %; (x) archivo corrupto o truncado → `opened=false` sin detener la corrida; (xi) un hijo que aborta (supervisor probado con un ejecutable de prueba) → cuenta como cierre inesperado; (xii) `.xlsm` con un `vbaProject.bin` ficticio → se abre y `macros_executed=false`; (xiii) dos corridas dan el mismo JSON salvo `run_id`. Además: `cargo run -p compat-check --release -- fixtures tests/fixtures/compat/ --out results/dev/compat/` imprime las cifras esperadas de los fixtures; `cargo test -p dokumentz-sheet --test no_ui_deps` sigue verde; `cargo tree -p compat-check` no muestra crates de interfaz; `cargo deny check licenses bans sources` limpio (incluye `zip`).
- [ ] T116 [US5] [MANUAL] Corrida de `compat-check` sobre A4 en macOS (requiere T114 con el visto bueno del dueño, T115 y, para ser oficial, T049; si faltara T049 los resultados van a `results/dev/` y SC-007 sigue `sin datos`): `bench-runner env` para registrar el hardware; `cargo run -p compat-check --release -- run --manifest testdata/a4/manifest.toml --dir testdata/a4/ --out results/macos-m1/<run-id>/` **dos veces** (el JSON debe coincidir salvo `run_id`); registrar IronCalc 0.9.1 y `calamine` 0.37.0. La persona revisa a mano una muestra de 10 discrepancias de (b) elegidas con semilla fija para comprobar que vienen del motor y no de la herramienta (comparando con ONLYOFFICE y, si está disponible, con Excel, sin habilitar macros), y revisa que el JSON versionado no contiene datos de los archivos del usuario. Los archivos se tratan como no confiables: ninguna macro se ejecuta. **Verificación:** existe `results/macos-m1/<run-id>/M-09-compat.json` con (a), (b), (c) en total, por fuente (usuario / Apache POI / LibreOffice) y por productor, y la lista (d); las dos corridas coinciden; `git status` no muestra nada bajo `testdata/a4/` salvo el manifiesto ni nada bajo `results/**/privado/`; `grep` de nombres de hoja y valores de los archivos del usuario sobre el JSON versionado no encuentra nada; la tabla de la muestra revisada queda en `results/macos-m1/<run-id>/muestra.md`.
- [ ] T117 [US5] Integrar M-09 y SC-007 en `bench-report`: (1) ampliar los tipos de `dokumentz-metrics::results` y `results.v1.schema.json` (T039) con `M-09-compat.json`; (2) leer `M-09-compat.json` y evaluar (a), (b), (c) con los umbrales de `criteria.toml` (ya congelados en T047), `cumple` / `no cumple` / `sin datos`, en total y por fuente; (3) implementar la **regla del motor** de `spec.md` §9 (M1, M2, M3 y `sin datos`) como módulo propio, **independiente** de la regla de interfaz; (4) secciones nuevas de `informe.md` e `informe.json`: tabla M-09 por fuente y por productor, las 20 funciones más frecuentes de (d), estado de SC-007 y rama del motor, separadas de la decisión de interfaz; (5) `sin datos` si falta el JSON, si A4 no está preparado o si no hay archivos producidos por Excel con fórmulas con valor guardado. **Verificación:** `cargo test -p bench-report compat` con JSON sintético: todo cumple → M1; (b) = 96 % → M2; (b) = 85 % → M3; sin archivos `excel` → `sin datos`; fronteras exactas (95 %, 98 %, 100 %, 90 %) y justo por debajo; un fallo de SC-007 no cambia la rama de la regla de interfaz (prueba cruzada) y viceversa; instantánea del Markdown; `cargo test -p bench-report schema` valida el ejemplo de `plan.md` §9.10 y rechaza uno con un campo requerido ausente.
- [ ] T118 [US5] Informe de funciones faltantes listo para abrir issues en IronCalc: subcomando `compat-check missing --in M-09-compat.json --out <carpeta>` que escribe `funciones-faltantes.md` y `.json`: por función, en el orden de M-09(d): nombre en inglés como en Excel, tipo (no soportada / resultado distinto), celdas y archivos afectados, un **ejemplo mínimo sintético** (fórmula con argumentos inventados, nunca datos de los archivos del usuario) con el resultado esperado tomado de la documentación oficial de la función (con su URL) o, si no se puede fijar con seguridad, marcado «esperado sin confirmar», y una plantilla de issue en inglés para https://github.com/ironcalc/IronCalc. **No abre ningún issue ni PR**: publicar en un repositorio ajeno requiere el visto bueno del dueño, que se pide con la lista. **Verificación:** `cargo test -p compat-check missing` con un JSON de entrada sintético (orden correcto, ejemplo generado, plantilla completa); prueba de privacidad: con una entrada que incluye archivos de la fuente `usuario`, el informe no contiene identificadores de archivo, valores de celda, nombres de hoja ni rutas; el dueño revisa la lista antes de cualquier publicación.

### CP-5 — US3, US5 y compatibilidad
**Corre:** los tres conjuntos de guiones en ambas apps; `cargo test -p dokumentz-sheet --test roundtrip`; `cargo run -p bench-runner --release -- roundtrip --dir testdata/`; `judge` sobre los archivos guardados; **`cargo test -p compat-check`** y `cargo run -p compat-check --release -- fixtures tests/fixtures/compat/ --out results/dev/compat/`; si A4 está preparado (T114), `cargo run -p compat-check --release -- run --manifest testdata/a4/manifest.toml --dir testdata/a4/ --out results/macos-m1/<run-id>/` y `cargo test -p bench-report compat`.
**Revisa:** que la ida y vuelta usa `calamine` (lector independiente) y no IronCalc para comparar; que el guardado es atómico; que el TSV lo arma el núcleo; que `compat-check` lee los valores guardados **solo** con `calamine`, compara contra el valor guardado (no contra otro recálculo del mismo motor), solo cuenta en (b) los archivos producidos por Excel, excluye y cuenta aparte las funciones volátiles, **nunca ejecuta macros**, y que el JSON versionado no lleva datos de los archivos del usuario.
**Decide:** SC-005 pasa o no (con los datos); si hay pérdida, se abre tarea o se registra la aclaración 8. **SC-007 y la regla del motor** (`spec.md` §9): cumple → motor validado; no cumple y (b) ≥ 90 % → plan de motor con la lista (d) (T118) para contribuir a IronCalc y volver a medir antes de la fase 1; (b) < 90 % → además, evaluar alternativas de motor de fórmulas; sin datos (A4 sin preparar) → el motor **no** se da por validado y se avisa al dueño.

---

## Fase 6 — US4: medir y comparar en las tres plataformas, y decidir la interfaz (P1) — FR-012, FR-013; M-06, M-08; SC-001…SC-006 (y SC-007 en la consolidación)

### E26 — CI completa
- **Agente:** trabajador (~90k). **Lee:** `plan.md` §10 y Anexo A; `.github/workflows/ci.yml`. **Toca:** `.github/workflows/ci.yml`, `tools/ci/*`; las apps solo si la CI lo exige.

- [ ] T081 [US4] CI completa de `plan.md` §10: trabajos `core`, `roundtrip`, `tauri`, `gpui` (y `judge-libreoffice` opcional) en la matriz de tres SO; paquetes de Linux (WebKitGTK; GPUI recortada de `script/linux` comprobando que compila); caché por SO; acciones fijadas por SHA. **Verificación:** `actionlint` sin errores; ejecución verde en macOS, Windows y Linux (**SC-006**); un resumen lista qué pruebas corrieron en cada SO y cuáles son de mejor esfuerzo.
- [ ] T082 [US4] E2E de ventana: `tauri-driver` en Linux (xvfb + WebKitWebDriver) y Windows (msedgedriver) con un recorrido de humo (abrir `testdata/ci/A1.xlsx`, desplazar, editar, deshacer, guardar); en macOS el humo usa el modo guion; humo de ventana de GPUI con `--script`. **Verificación:** trabajo de CI verde; en macOS el humo pasa con `dokumentz-script-check`.

### E27 — Runbook y preparación en macOS
- **Agente:** trabajador para T083 y T085, persona para T084 (~60k). **Lee:** `plan.md` §9; `tools/bench-runner/apps.toml`. **Toca:** `docs/medicion/RUNBOOK.md`, `tools/bench-runner/apps.toml`, `results/macos-m1/<run-id>/`.

- [ ] T083 [US4] `docs/medicion/RUNBOOK.md`: pasos exactos por plataforma (instalar `ffmpeg` y PresentMon, permisos, sesión X11, ajustes de pantalla, vaciado de caché con privilegios, ONLYOFFICE sin actualizaciones, colocación de ventanas, lectura del informe), con los comandos de `bench-runner`. **Verificación:** un agente `mecanico` ejecuta los pasos de macOS al pie de la letra y todos funcionan sin ayuda; las discrepancias se corrigen en el RUNBOOK.
- [ ] T084 [US4] [MANUAL] macOS: conceder Grabación de pantalla y Accesibilidad al terminal; correr `bench-runner env` y `preflight`; comprobar que ONLYOFFICE abre un `.xlsx` pasado como argumento; fijar en `apps.toml` las coordenadas de cuadrícula, pista de la barra y barra de fórmulas de las tres apps con capturas de verificación; `bench-runner calibrate --target dokumentz-app-gpui --calibrate`. **Verificación:** `results/macos-m1/<run>/calibracion.json` con `floor_ms`; `apps.toml` con las tres apps y sus capturas.
- [ ] T085 [US4] macOS: compilar ambas apps empaquetadas en `release` (`npm run tauri build`; binario de GPUI dentro de un `.app` mínimo) y medir M-08 con `du -sk` también para `ONLYOFFICE.app`. **Verificación:** JSON de M-08 de las tres apps con la nota de salvedades (la webview del sistema no se incluye; alcance de ONLYOFFICE); instaladores aparte.

### E28 — Medición externa en macOS
- **Agente:** mecanico, el principal revisa (~60k). **Requiere:** T049 hecha (criterios congelados) y T084. **Lee:** `RUNBOOK.md`, `apps.toml`. **Toca:** `results/macos-m1/<run-id>/**`.

- [ ] T086 [US4] macOS: M-01 (variante B) y M-01-inicio, y M-02 con A1; 5 repeticiones, frío y caliente, tres apps. **Verificación:** JSON válidos; capturas de verificación guardadas; ninguna repetición descartada sin registro.
- [ ] T087 [US4] macOS: M-03 (S1 rueda y S2 barra), 5 repeticiones, tres apps. **Verificación:** JSON con p95, p99, cuadros perdidos y fps efectivos; frecuencia de refresco registrada (60 Hz).
- [ ] T088 [US4] macOS: M-04 (A1 y A2) y M-05 (A2), 5 repeticiones, tres apps. **Verificación:** JSON; una M-05 en la que las dependientes no cambian es inválida y se repite.
- [ ] T089 [US4] macOS: M-06 con A1 (sin captura de pantalla) y validación de la atribución de procesos WebKit (lista de PIDs antes y después con `footprint -a`; cierre de la app). **Verificación:** JSON con `value_mb`, `peak_mb` y la lista de PIDs sumados por app; los servicios WebKit nuevos desaparecen al cerrar PoC-Tauri.
- [ ] T090 [US4] macOS: M-07 (guardar A1 tras editar una celda) con comprobación de integridad por `xlsx-diff`, frío y caliente, tres apps. **Verificación:** JSON; los fallos de integridad se cuentan como fallo.
- [ ] T091 [P] [US4] macOS: verificación cruzada opcional de M-03 con `xcrun xctrace record --template "Animation Hitches"` sobre las dos PoC y ONLYOFFICE. **Verificación:** archivos `.trace` en `results/…/evidencia/` y una nota que compara el orden de magnitud con la captura de pantalla.
- [ ] T092 [US4] macOS: `bench-report` sobre la carpeta de resultados → `informe.md` parcial (solo macOS). **Verificación:** informe con SC-001…SC-003 evaluados en macOS y `sin datos` en Windows y Linux.

### CP-6 — CI completa y medición en macOS
**Corre:** la CI en las tres plataformas (SC-006); `cargo run -p bench-report -- --results results/macos-m1/<run> --criteria tools/bench-report/criteria.toml` y compara con el informe del agente; `cargo test-core`.
**Revisa:** `criteria_sha` en todos los JSON; ninguna repetición descartada en silencio; las capturas de verificación muestran la app correcta en la región correcta; la nota fija de alcance de ONLYOFFICE está en el informe; ningún JSON mezcla `method: internal` con `external`.
**Decide:** si M-03/M-04 externos contradicen a los internos de forma grande, se abre tarea de diagnóstico antes de medir en otras plataformas; confirma con el dueño el estado del hardware (aclaración 2).

---

### E29 — Medición en Windows `[HW-Win]`
- **Agente:** mecanico + persona (~60k). **Bloqueada** hasta tener el equipo (aclaración 2). **Lee:** `RUNBOOK.md` (sección Windows), `apps.toml`. **Toca:** `results/windows-<equipo>/<run-id>/**`, `tools/bench-runner/apps.toml`.

- [ ] T093 [HW-Win] Preparar el equipo: Rust 1.99.0, Node, `ffmpeg` con `ddagrab`, PresentMon, ONLYOFFICE (versión registrada), WebView2, clonar el repositorio, `cargo test-core` y compilar `release` ambas apps. **Verificación:** pruebas verdes; `bench-runner env` imprime el hardware real (modelo, GPU, versión de WebView2).
- [ ] T094 [HW-Win] Calibración y M-01/M-02 con A1 (frío con reinicio o RAMMap). **Verificación:** JSON en `results/windows-<equipo>/<run>/`.
- [ ] T095 [HW-Win] M-03 (S1 y S2) con verificación cruzada de PresentMon filtrando el árbol de procesos de cada app. **Verificación:** JSON + CSV de PresentMon; nota de coincidencia o discrepancia.
- [ ] T096 [HW-Win] M-04 y M-05. **Verificación:** JSON.
- [ ] T097 [HW-Win] M-06 (conjunto de trabajo privado del árbol), M-07 y M-08. **Verificación:** JSON con los PIDs sumados.
- [ ] T098 [HW-Win] [MANUAL] SC-004 en Windows con IME japonés/chino y teclas muertas en ambas apps (`tests/manual/fr007-ime.md`). **Verificación:** tabla de Windows rellena.

### E30 — Medición en Linux `[HW-Linux]`
- **Agente:** mecanico + persona (~70k). **Bloqueada** hasta tener el equipo. **Lee:** `RUNBOOK.md` (sección Linux). **Toca:** `results/linux-<equipo>/<run-id>/**`.

- [ ] T099 [HW-Linux] Preparar el equipo con sesión **X11** (distribución y versión registradas), los paquetes de `plan.md` §10, `ffmpeg` con `x11grab`, `xdotool`, ONLYOFFICE y el controlador de la GPU. **Verificación:** pruebas verdes; `bench-runner env` con GPU, controlador y versión de WebKitGTK.
- [ ] T100 [HW-Linux] Calibración y M-01/M-02. **Verificación:** JSON.
- [ ] T101 [HW-Linux] M-03 (S1 y S2); MangoHud opcional. **Verificación:** JSON.
- [ ] T102 [HW-Linux] M-04, M-05, M-06 (`Pss`), M-07 y M-08. **Verificación:** JSON.
- [ ] T103 [HW-Linux] [MANUAL] SC-004 en Linux (IBus o Fcitx) en ambas apps. **Verificación:** tabla de Linux rellena.
- [ ] T104 [HW-Linux-NVIDIA] Repetir M-01…M-04 en un equipo con NVIDIA **sin** mitigaciones y **con** cada mitigación documentada (`WEBKIT_DISABLE_DMABUF_RENDERER=1`, etc., D-08), registrando la configuración exacta. **Verificación:** JSON con `env.mitigations` y observaciones (ventana en blanco, parpadeo, cierres).
- [ ] T105 [HW-120Hz] Repetir M-03 en macOS con una pantalla ProMotion de 120 Hz si hay una disponible (aclaración 2). **Verificación:** JSON con `refresh_hz` = 120; si no hay pantalla, el ideal de SC-002 queda `sin datos`.

### E31 — Consolidación
- **Agente:** trabajador (~60k). **Lee:** `results/*/`, `spec.md` §8–§9, `plan.md` §9.8. **Toca:** `results/informe-consolidado.*`.

- [ ] T106 [P] [US4] A4 (ida y vuelta y jueces; la compatibilidad de valores con Excel es T116): pasar archivos reales del usuario por `roundtrip` y `judge` en local (nunca se suben a internet). **Verificación:** tabla de resultados sin rutas ni contenido en el repositorio.
- [ ] T107 [US4] Informe consolidado: `bench-report` sobre las carpetas de las tres plataformas; evaluar SC-001…SC-007 y aplicar, por separado, la regla de decisión de interfaz y la regla del motor (M-09 de T116); marcar `sin datos` lo no medido; incluir «dónde pierde la PoC». **Verificación:** `results/informe-consolidado.md` generado por la herramienta; cada SC con su dato; la rama de la regla nombrada.

### CP-7 — Datos completos o declarados incompletos
**Corre:** `bench-report` de nuevo sobre `results/`; compara con el informe del agente.
**Revisa:** que lo no medido figura `sin datos`; que no hay extrapolación entre plataformas; que Linux/NVIDIA indica la configuración; que todos los `criteria_sha` coinciden con `criterios-v1`.
**Decide:** si faltan plataformas, el dueño decide si se cierra la PoC con datos parciales (la regla de decisión no se da por aplicada) o se espera al hardware.

---

## Fase 7 — Cierre

### E32 — Cierre
- **Agente:** mecanico (T108, T109), trabajador (T111, T112), persona (T110, T113) (~60k). **Lee:** `results/`, `Cargo.lock`, `package-lock.json`, `research.md`, `spec.md`. **Toca:** `research.md`, `spec.md` (solo estado de aclaraciones), `plan.md` (marcas), `docs/procedencia.md`, `specs/001-poc-hoja-de-calculo/decision.md`.

- [ ] T108 Auditoría de licencias completa: `cargo deny check licenses bans sources` sobre el `Cargo.lock` final (incluye la clausura de GPUI) y revisión de licencias de `npm` (`license-checker` o equivalente) en `apps/tauri`. **Verificación:** ninguna AGPL ni licencia desconocida; `research.md` D-15 actualizado con el resultado.
- [ ] T109 Comprobación sin red (principio I): ejecutar ambas apps en una sesión (abrir, desplazar, editar, guardar) con la red apagada y revisando las conexiones (`lsof -i -a -p <pid>` o `nettop` en macOS; `Get-NetTCPConnection` en Windows; `ss -tp` en Linux). **Verificación:** cero conexiones salientes y la sesión funciona completa; registro en `results/`.
- [ ] T110 Registro de decisión: `specs/001-poc-hoja-de-calculo/decision.md` con el informe consolidado, la rama de la regla, los `sin datos`, la propuesta de interfaz y el espacio para la decisión del dueño (obligatoria si el caso es «decisión del usuario» o «no cubierto»). **Verificación:** el dueño aprueba por escrito; **EX-01** se cierra o se renueva; la licencia del proyecto (aclaración 1) queda resuelta según la regla 6 del Gobierno.
- [ ] T111 Registro de procedencia (principio V): `docs/procedencia.md` con una fila por recurso gráfico, fuente y código adaptado (icono propio de la app, fuentes del sistema, adaptación de `input.rs` de Zed). **Verificación:** cada recurso tiene origen y licencia; ninguno proviene de un producto de Microsoft.
- [ ] T112 Actualizar documentos: `research.md` («Hechos sin verificar» con lo resuelto), `spec.md` (aclaraciones cerradas), `plan.md` (todas las marcas `[A VERIFICAR EN Txxx]` resueltas) y la nota de accesibilidad para la fase 1. **Verificación:** `grep -rn "A VERIFICAR EN" specs/` no devuelve marcas sin resolver o sin tarea de seguimiento.
- [ ] T113 Cierre de la PoC: CI verde en las tres plataformas (SC-006), etiqueta `poc-001-cerrada`, lista de pendientes para la fase 1 y recomendación de `/clear` y sesión nueva. **Verificación:** enlace a la ejecución de CI verde dentro de `decision.md`.

### CP-8 — Cierre con el dueño
**Corre:** CI completa; `cargo deny check`; `grep -rn "NECESITA ACLARACIÓN" specs/` para listar lo que sigue abierto.
**Revisa:** que la decisión de interfaz se apoya en `results/informe-consolidado.md` y en la regla fijada antes de medir; que las medidas en que la PoC pierde están en el informe.
**Decide el dueño:** interfaz para la fase 1 (o detener, o decidir entre Tauri y GPUI con los datos); licencia del proyecto; qué hacer con las plataformas sin datos.

---

## Trazabilidad

| Historia | Requisitos | Tauri | GPUI | Núcleo y herramientas | Medidas |
|---|---|---|---|---|---|
| US1 (P1) | FR-001…FR-004 | T050–T056 | T057–T061 | T015–T022, T032 | M-01, M-02, M-03: T063 (interna), T086, T087 (externas) |
| US2 (P1) | FR-005…FR-008 | T064–T066 | T067–T069 | T023–T025 | M-04, M-05: T071 (interna), T088 (externas) |
| US3 (P2) | FR-009…FR-011 | T073, T074, T076 | T073, T075, T076 | T026–T028, T077–T078 | M-07: T079 (interna), T090 (externa) |
| US4 (P1) | FR-012, FR-013 | T056 | T061 | T012–T014, T033–T049 | M-06, M-08: T085, T089; Windows y Linux: T093–T105; informe y decisión: T107–T110 |
| US5 (P1) | FR-014 | — (no depende de ninguna interfaz) | — | T114 (A4), T115 (`compat-check`), T118 (funciones faltantes); T036 (`calamine`) | M-09: T116 (corrida sobre A4), T117 (informe y regla del motor) |

| Criterio | Tareas que lo evalúan |
|---|---|
| SC-001 | T086, T089, T107 (M-02 y M-06 en cada plataforma) |
| SC-002 | T087, T095, T101, T105, T107 |
| SC-003 | T088, T096, T102, T107 |
| SC-004 | T008 (spike), T072, T098, T103 |
| SC-005 | T038, T077, T078, T106 |
| SC-006 | T011, T081, T082, T113 |
| SC-007 | T114, T115, T116, T117, T118, T107 (umbrales congelados en T047/T049; regla del motor en T117) |

**Tareas dependientes de hardware real (bloqueadas hasta la aclaración 2):** T093–T098 (Windows), T099–T104 (Linux; T104 además con GPU NVIDIA), T105 (pantalla de 120 Hz). Mientras tanto la CI (T081–T082) y las VM solo prueban lo funcional.

**Tareas que esperan una decisión del dueño:** CP-0 (confirmar el primer `push` al repositorio público), T114 (visto bueno para descargar los corpus públicos; sin él, T116 y SC-007 quedan `sin datos`), T118 (visto bueno antes de abrir issues o enviar cambios a IronCalc), T110 (decisión de interfaz). Tres aplicaciones como Office (decidido el 2026-10-22, 000 §4.4); no cambia esta PoC. T049 solo espera a que exista git. Resueltas el 2026-10-22: licencia (MIT OR Apache-2.0), hardware (solo Mac por ahora), repositorio (GitHub público) y EX-01 (aprobada).
