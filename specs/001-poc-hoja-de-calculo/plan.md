# Plan de implementación 001 — PoC de hoja de cálculo

| Campo | Valor |
|---|---|
| Estado | Borrador para revisión del agente principal y del dueño del proyecto |
| Fecha | 2026-10-22 |
| Rama de especificación | `001-poc-hoja-de-calculo` |
| Documentos hermanos | `specs/constitucion.md` (prevalece sobre este plan), `specs/000-arquitectura-suite/arquitectura.md` (**entrada**: arquitectura de la suite y hoja de ruta; esta PoC es su fase 0a), `spec.md` (QUÉ y POR QUÉ), `research.md` (decisiones y fuentes), `tasks.md` (tareas y etapas) |
| IDs que usa este plan | `FR-001…FR-014`, `M-01…M-09`, `SC-001…SC-007`, `A1…A4`, `US1…US5` (iguales a `spec.md`); `D-xx` remite a `research.md`; `R-xx` son riesgos de este plan; `EX-xx` son excepciones a la constitución; `CG-x` son compuertas de decisión temprana |
| Convención de verificación | `[VERIFICADO]` = comprobado en esta sesión con la URL o el comando indicado (anexo A). `[NO VERIFICADO]` = dicho de memoria o inferido; no se debe apoyar trabajo grande en ello sin la tarea que lo comprueba. `[A VERIFICAR EN Txxx]` = lo resuelve una tarea de `tasks.md` |

---

## 1. Resumen

Este plan parte de la **arquitectura de la suite** (`specs/000-arquitectura-suite/arquitectura.md`): Dokumentz será una suite (hoja, documentos, presentaciones) con una base común y un motor por tipo de documento, e IronCalc es solo la calculadora de la hoja. Esta PoC es la **fase 0a** de esa hoja de ruta y **construye piezas de producto**, no un instrumento desechable: `dokumentz-sheet`, `dokumentz-metrics`, `dokumentz-script` y las herramientas `tools/*` se conservan; solo la interfaz perdedora se descarta. Las demás piezas de la base común (`dokumentz-opc`, `dokumentz-text`, `dokumentz-render`, `dokumentz-edit`, `dokumentz-shell`…) **no** se crean aquí: llegan en la fase 1 (ver nota del §4).

La PoC construye **un solo núcleo en Rust** (`dokumentz-sheet`, que envuelve `ironcalc_base::UserModel`) y **dos interfaces delgadas** sobre él: **Tauri 2** (webview del sistema, cuadrícula en canvas 2D) y **GPUI** (nativa con GPU, cuadrícula como elemento de dibujo propio). Ambas ofrecen exactamente FR-001…FR-013: abrir `.xlsx`, mostrar la cuadrícula con estilos, desplazarse con fluidez en 100 000+ filas, navegar, editar con fórmulas en español e inglés, IME, deshacer/rehacer, varias hojas, copiar/pegar tabulado y guardar `.xlsx`. Sin cinta.

El núcleo no sabe nada de ventanas. Cada cuadro que una interfaz dibuja sale de una **lista de dibujo** (`DrawList`): las celdas del área visible más un margen, con texto ya formateado, estilo ya resuelto y geometría ya calculada. En Tauri la lista viaja en **binario** (`tauri::ipc::Response`), se guarda en una caché en la webview y el desplazamiento **no hace llamadas al puente** mientras el área visible siga dentro del bloque en caché. En GPUI la lista se lee directamente del núcleo en el mismo proceso.

La PoC mide las tres aplicaciones (PoC-Tauri, PoC-GPUI y ONLYOFFICE 10.0.0) con **el mismo método externo** (captura de pantalla a tasa constante + entrada sintética del sistema + muestreo de memoria de todos los procesos) y, además, a las dos PoC con su **instrumentación interna**. Un orquestador (`bench-runner`) produce JSON; un agregador (`bench-report`) calcula medianas y p95, evalúa SC-001…SC-007, aplica la regla de decisión de interfaz de `spec.md` §9 y, **por separado**, la regla del motor, y escribe el informe. Los criterios se congelan **antes** de la primera corrida oficial.

Además, la herramienta `tools/compat-check` responde la pregunta 5 de la spec (¿el motor abre los archivos reales con los mismos resultados que Excel?): abre cada archivo de A4 con el motor de la hoja, recalcula y compara con el valor que el propio archivo trae guardado, con `calamine` como lector independiente (US5, FR-014, M-09, SC-007; método en §9.10 y D-17). No mide tiempo y no depende de ninguna interfaz.

Cuatro hallazgos de esta sesión condicionan el diseño y están marcados en el texto:

1. **IronCalc reevalúa todo el libro en cada edición.** En el código de `main`, `UserModel::set_user_input` llama a `Model::evaluate()`, cuya documentación dice «Evaluates every formula in the workbook», y `evaluate` recorre todas las fórmulas y, en las hojas con fórmulas, todas las celdas buscando matrices dinámicas. No es incremental. Afecta a M-04 y M-05 en A2 (riesgo R-04). `[VERIFICADO]` en `main` **y en la etiqueta `v0.9.1`** (revisión del 2026-10-22: en `base/src/evaluation.rs`, `run_pass` vacía la caché de celdas y evalúa todas las `formula_positions()` en cada pasada; `evaluate_if_not_paused` llama a `self.model.evaluate()`).
2. **Con idioma `es`, los nombres de función en inglés no se aceptan** (`=TRUE()` da `#¿NOMBRE?`; las fórmulas se guardan internamente en inglés y se muestran en el idioma activo). FR-006 exige aceptar ambos idiomas, así que el núcleo necesita una política propia (§5.7). `[VERIFICADO]` en las pruebas de `main`.
3. **GPUI como dependencia git exige replicar los `[patch.crates-io]` del `Cargo.toml` de Zed** (`async-task`, `calloop`, `khronos-egl`…): la lista `[VERIFICADO]` en el `Cargo.toml` de Zed; que Cargo no aplica los `[patch]` de una dependencia a quien la consume es comportamiento conocido de Cargo, `[NO VERIFICADO]` en esta sesión y comprobado de todos modos en T007.
4. **El campo `Application` de `docProps/app.xml` no distingue a Excel de las bibliotecas que copian su nombre.** `rust_xlsxwriter` 0.101.0 escribe «Microsoft Excel», `AppVersion` 12.0000 y `fullCalcOnLoad="1"`, y XlsxWriter (Python) escribe la misma cadena. La regla decidida para SC-007 usa `Application`; el límite y la propuesta de una segunda señal están en §9.9 y §9.10. `[VERIFICADO]` en un archivo generado localmente y en https://raw.githubusercontent.com/jmcnamara/XlsxWriter/main/xlsxwriter/app.py.

---

## 2. Contexto técnico

**Entrada de arquitectura.** `specs/000-arquitectura-suite/arquitectura.md` fija las capas de la suite (base común, motores por tipo de documento, interfaz y herramientas), los nombres de producto (D-18) y la hoja de ruta 0a/0b/1…5. De ahí salen, para esta PoC: (1) los nombres de crates y carpetas del §4; (2) que `dokumentz-sheet` envuelve a IronCalc pero no es «el motor de la suite» (D-16); (3) que cada fase mide velocidad **y** compatibilidad, por lo que la PoC incorpora SC-007; (4) que la lista de dibujo de la cuadrícula (§5.4) es específica de la hoja y será el primer cliente del futuro `dokumentz-render`; (5) que EX-01 (texto en la interfaz) se cierra cuando entre `dokumentz-text` en la fase 1.

### 2.1 Lenguajes y toolchain

| Elemento | Decisión | Estado |
|---|---|---|
| Rust | **1.99.0** fijado en `rust-toolchain.toml` (con `rustfmt` y `clippy`), edición 2024, `resolver = "3"` | Instalado en el Mac del usuario (encargo). Zed usa edición 2024 y toolchain 1.98.1 `[VERIFICADO]` (raw.githubusercontent.com/zed-industries/zed/main/Cargo.toml y rust-toolchain.toml), por lo que 1.99.0 es suficiente para GPUI |
| TypeScript | Solo en `apps/tauri/src`, sin framework (DOM + canvas 2D). Empaquetado con Vite, pruebas con Vitest | Versiones exactas: `package-lock.json` en T009. `[NO VERIFICADO]` qué versiones resuelve npm hoy |
| Node | 22.23.3 (solo herramientas de frontend y `@tauri-apps/cli`) | Instalado en el Mac |
| Swift / Objective-C | No se escribe en la PoC. El método externo usa `ffmpeg` y la API `CGEvent` desde Rust | `[A VERIFICAR EN T042]`; plan B: ayudante en Swift si `ffmpeg` no alcanza la tasa de cuadros en macOS |
| Perfil `release` común a ambas apps | `opt-level = 3`, `lto = "thin"`, `codegen-units = 1`. **Nunca se mide una compilación `debug`** | Decisión del plan |

### 2.2 Dependencias con versión

| Dependencia | Versión / fijación | Uso | Estado |
|---|---|---|---|
| `ironcalc` y `ironcalc_base` | `=0.9.1` | Motor, importar/exportar `.xlsx` | `[VERIFICADO]` 0.9.1 en docs.rs (`ironcalc_base::expressions::types::Area`) |
| `rust_xlsxwriter` | `=0.101.0` (MIT OR Apache-2.0) | El generador (FR-013) y los fixtures de prueba de `xlsx-diff` y de `compat-check`; es independiente de IronCalc a propósito: así la importación de IronCalc se prueba contra un escritor ajeno | `[VERIFICADO]` docs.rs; `Formula::new(..).set_result(..)` y `Worksheet::set_formula_result(..)` existen `[VERIFICADO]` (docs.rs y prueba local del 2026-10-22: el valor guardado forzado se lee tal cual) |
| `calamine` | **`=0.37.0`** (MIT; publicada el 2026-10-18) | Lector independiente de IronCalc: ida y vuelta (SC-005, `xlsx-diff`) y valores guardados y fórmulas de los archivos reales (SC-007, `compat-check`) | `[VERIFICADO]` crates.io (versión y licencia) y docs.rs; `worksheet_range` devuelve el **valor guardado** de las celdas con fórmula y `worksheet_formula` su texto, probado localmente con 0.37.0 el 2026-10-22 (D-17) |
| `zip` | La que resuelva `calamine` en `Cargo.lock` (9.0.5 en la prueba local del 2026-10-22) | Solo `compat-check`: leer `docProps/app.xml` para detectar el productor del archivo | `[NO VERIFICADO]` su licencia; la comprueba `cargo deny` (T003, T108) |
| `tauri` | `~2.12` (estable 2.12.5, 21-oct-2026) | Interfaz A | `[VERIFICADO]` github.com/tauri-apps/tauri/releases |
| `@tauri-apps/cli` | `~2.12` (2.12.4 listado el 21-oct-2026) | Compilar y empaquetar | `[VERIFICADO]` ídem |
| `@tauri-apps/api` | `2.x` | `invoke`, `Channel` | `[VERIFICADO]` la clase `Channel<T>` en `packages/api/src/core.ts` |
| `tauri-plugin-dialog` | `2.x` | Diálogos de abrir/guardar en Tauri | `[VERIFICADO]` nombre, permisos `dialog:allow-open`/`dialog:allow-save` y API (v2.tauri.app/plugin/dialog/) |
| `tauri-plugin-clipboard-manager` | `2.x` (exige Rust ≥ 1.90) | Portapapeles de texto en Tauri | `[VERIFICADO]` (v2.tauri.app/plugin/clipboard/). Permisos `clipboard-manager:allow-read-text` y `allow-write-text` |
| `gpui` y `gpui_platform` | **Git** `https://github.com/zed-industries/zed` con `rev` fijado. Candidato: `9e84b066b55bc3609a30dd965075c12255b38b45` (cabeza de `main` el 22-oct-2026); se confirma en T007 compilando en las tres plataformas. Características de `gpui_platform`: `font-kit`, `wayland`, `x11`; en macOS `runtime_shaders` solo si no hay toolchain de Metal | Interfaz B (nativa con GPU) | `[VERIFICADO]` README y `Cargo.toml` de `crates/gpui` y `crates/gpui_platform`. La versión de crates.io (0.2.2) está obsoleta |
| Parches de Zed | Replicar en el `Cargo.toml` raíz los `[patch.crates-io]` que afecten a la clausura de `gpui` (`async-task`, `calloop`, `khronos-egl`; `windows-capture` solo si se activa `screen-capture`, que no se activa), con los `rev` del commit elegido | Compilar GPUI como dependencia git | `[VERIFICADO]` la lista en `Cargo.toml` de Zed; `[A VERIFICAR EN T007]` cuáles aplican |
| Crates internos de Zed bajo `gpui_platform` | Todos Apache-2.0 en `main` (`gpui`, `gpui_platform`, `gpui_macos`, `gpui_windows`, `gpui_linux`, `gpui_wgpu`, `gpui_web`, `gpui_ios`, `gpui_apple`, `gpui_macros`, `gpui_shared_string`, `gpui_util`, `collections`, `http_client`, `refineable`, `derive_refineable`, `scheduler`, `sum_tree`, `util`, `util_macros`, `path`, `perf`, `zlog`, `ztracing`, `ztracing_macro`, `bench_metrics`) | Licencia de la clausura interna de GPUI | `[VERIFICADO]` leyendo el `Cargo.toml` de cada uno. Las dependencias de terceros se auditan con `cargo deny` (T003, T108) |
| `serde`, `serde_json`, `thiserror`, `rand_chacha`, `sha2`, `clap` | Última estable; se fijan en `Cargo.lock` | Núcleo, medición, herramientas | `[NO VERIFICADO]` versiones |
| `ffmpeg` | Externo, no se distribuye. Debe incluir el filtro `ddagrab` (Windows), `x11grab` (Linux) y `avfoundation` (macOS) | Captura de pantalla del método externo | `[VERIFICADO]` que esos dispositivos y filtros existen en ffmpeg.org/ffmpeg-devices.html y ffmpeg-filters.html. La versión mínima se fija en T042 |
| PresentMon | 2.x (CLI) | Verificación cruzada de cuadros en Windows | `[VERIFICADO]` opciones y columnas en el README de la consola del repo `GameTechDev/PresentMon` |
| `x2t` de ONLYOFFICE | 10.0.0.169, incluido en `ONLYOFFICE.app/Contents/Resources/converter/x2t` | Juez de compatibilidad automático | `[VERIFICADO]` localmente (`x2t` sin argumentos muestra «OOX/binary file converter. Version: 10.0.0.169» y «conversion direction will be calculated from file extensions») |

**Regla de dependencias.** Ninguna con `*`. Todo lo que no tenga versión en esta tabla se fija con `Cargo.lock` / `package-lock.json` en la tarea que lo introduce. Ninguna dependencia nueva entra sin pasar `cargo deny check licenses bans` (principio I).

### 2.3 Almacenamiento

- **Documento:** un `.xlsx` en disco (formato nativo; principio IV). No hay base de datos, ni formato propio, ni caché persistente. El estado en memoria es el `UserModel` de IronCalc.
- **Guardado atómico:** el núcleo escribe a un archivo temporal en la misma carpeta y renombra al terminar; si falla (disco lleno, sin permisos) el original queda intacto (caso límite de `spec.md` §4).
- **Resultados de medición:** JSON en `results/<plataforma>/<run-id>/` (esquema en §9.7). Informe generado: `informe.md` e `informe.json` en la misma carpeta.
- **Archivos de prueba:** A1–A3 generados por `testdata-gen` en `testdata/` (ignorado por git; se regeneran con semilla fija y se verifica su `sha256` de contenido en `manifest.json`). Solo viven en el repositorio los fixtures pequeños (≤ 100 KB) para pruebas unitarias. A4 nunca se copia al repositorio ni sale del equipo.
- **Resultados de compatibilidad (M-09):** `results/<plataforma>/<run-id>/M-09-compat.json` y `compat.md` (§9.10). Como `results/` puede subirse al repositorio público, el JSON versionado de los archivos de la fuente «usuario» **no** lleva valores de celda, nombres de hoja ni rutas (solo un identificador derivado del SHA-256, recuentos y nombres de función); el detalle celda a celda va a `results/<plataforma>/<run-id>/privado/`, ignorado por git (T001).
- **Sin configuración persistente** en la PoC (ni preferencias, ni telemetría, ni cuentas).

### 2.4 Pruebas

| Nivel | Herramienta | Dónde corre |
|---|---|---|
| Núcleo (unidad, integración, doradas) | `cargo test -p dokumentz-sheet`; archivo dorado `crates/dokumentz-sheet/tests/golden/drawlist_v1.bin` compartido con el decodificador de TypeScript | CI en 3 SO; sin ventana |
| Grafo de dependencias del núcleo | Prueba `crates/dokumentz-sheet/tests/no_ui_deps.rs` que usa `cargo metadata` y falla si aparece cualquier crate de una lista negra (`tauri*`, `wry`, `tao`, `gpui*`, `gtk*`, `webkit2gtk*`, `winit`, `objc2-app-kit`…) | CI en Linux |
| Frontend de Tauri | Vitest + `mockIPC` de `@tauri-apps/api/mocks` `[VERIFICADO]` (v2.tauri.app/develop/tests/mocking/) | CI en 3 SO |
| Capa de comandos de Tauri | `cargo test -p dokumentz-app-tauri` llamando a las funciones de `commands.rs` con un `Core` real (sin runtime de Tauri) | CI en 3 SO |
| E2E de ventana de Tauri | `tauri-driver` solo en Windows y Linux (en macOS no hay driver de WKWebView) `[VERIFICADO]` (v2.tauri.app/develop/tests/webdriver/); en macOS se usa el **modo guion** (§5.10) | CI: Linux (xvfb) y Windows; macOS por guion |
| GPUI | `gpui` con característica `test-support` y `TestAppContext` para lógica de entrada y teclas sin GPU `[VERIFICADO]` en el README de `gpui`; humo de ventana con modo guion | CI: pruebas sin GPU en 3 SO; humo de ventana en Linux con xvfb y Vulkan por software, **mejor esfuerzo** |
| Funcionales comunes (FR-001…FR-011) | Guiones JSON en `tests/functional/` ejecutados por **ambas** apps con `--script` y verificados por `dokumentz-script-check` | CI en 3 SO + local |
| Ida y vuelta (SC-005) | `xlsx-diff` (calamine) sobre A1–A3; juez `x2t` y LibreOffice headless | Local (completo); CI con versión reducida `--scale ci` |
| Compatibilidad del motor (SC-007) | `cargo test -p compat-check` con fixtures de `rust_xlsxwriter` + `set_formula_result` (valor guardado coincidente y discrepante, volátiles, sin valor guardado, productor Excel y no Excel, archivo corrupto, `.xlsm` ficticio); corrida real sobre A4 con `compat-check run` | Autoprueba con fixtures: CI en 3 SO. Corrida sobre A4: local (macOS), nunca en CI porque A4 no sale del equipo |
| Manual guiado (SC-004) | `tests/manual/fr007-ime.md` con tabla de resultados por plataforma | Persona, en hardware real |

### 2.5 Plataformas

| Plataforma | Objetivo de rendimiento | Objetivo funcional/CI |
|---|---|---|
| macOS | Apple M1, 16 GB, macOS 26.6.2 (pantalla Retina 2560×1600, 60 Hz; **no** es ProMotion) `[VERIFICADO]` con `system_profiler` | `macos-26` (arm64) |
| Windows | **Hardware real pendiente** (aclaración 2 de `spec.md`). WebView2 evergreen | `windows-2025` |
| Linux | **Hardware real pendiente**, idealmente con NVIDIA. Distribución de referencia pendiente; se propone Ubuntu 24.04 con **sesión X11** para la medición (la inyección de entrada con XTest no funciona en clientes Wayland nativos `[NO VERIFICADO]`) | `ubuntu-24.04` |

Etiquetas de ejecutores `[VERIFICADO]` (github.com/actions/runner-images README): `ubuntu-24.04`, `windows-2025`, `macos-26` (arm64). Los ejecutores estándar no documentan GPU `[VERIFICADO]` (docs.github.com), por lo que la CI solo hace pruebas funcionales (D-13).

### 2.6 Metas de rendimiento (las de `spec.md`)

| Medida | Meta / criterio | Presupuesto interno de diagnóstico (propuesta del plan; no sustituye a SC) |
|---|---|---|
| M-01 arranque | Sin umbral SC. Se reporta con y sin archivo (§9.3) | Núcleo listo y primera ventana pintada ≤ 400 ms en el Mac (propuesta) |
| M-02 abrir A1 | SC-001: mediana de la mejor interfaz ≤ ½ de ONLYOFFICE | `load_from_xlsx` de A1 es lo dominante; se mide en T005 (CG-1) |
| M-03 desplazar | SC-002: p95 ≤ 16,7 ms en las tres plataformas; ideal ≤ 8,3 ms en ProMotion si hay pantalla | `DrawList` de pantalla + margen ≤ 3 ms; ida y vuelta del puente (Tauri, 100 KB) p95 ≤ 5 ms; pintado de canvas ≤ 6 ms |
| M-04 tecleo | SC-003: p95 ≤ 30 ms | Evento → cuadro presentado ≤ 16 ms internos |
| M-05 recálculo | Sin umbral SC; se reporta (hipótesis: afectada por R-04) | `Core::apply(CommitEdit)` sobre A2 se mide aislado en T031 |
| M-06 memoria | SC-001: ≤ 50 % de la de ONLYOFFICE | RSS del núcleo con A1 abierto, medido en T005 (CG-1) |
| M-07 guardar | Sin umbral SC; solo vale si la integridad pasa (SC-005) | `save_to_xlsx` de A1 medido en T005 |
| M-08 tamaño | Sin umbral SC; se informa con la salvedad de alcance de ONLYOFFICE | Binario release de cada app |
| M-09 compatibilidad | SC-007: (a) ≥ 95 % de archivos abren sin error y 0 cierres; (b) ≥ 98 % de fórmulas coinciden con Excel (archivos producidos por Excel); (c) 100 % de valores sin fórmula idénticos; por fuente además | No es tiempo: determinista, una corrida por archivo (§9.10) |

### 2.7 Restricciones del plan

- Sin red, sin cuentas, sin telemetría. La CSP de Tauri prohíbe cualquier origen externo (T051). La instrumentación está apagada por defecto y solo se activa con `--metrics-out <ruta>` o `DOKUMENTZ_METRICS_OUT`.
- No se copia código de ONLYOFFICE (principio VII). El ejemplo `input.rs` de Zed (Apache-2.0) sí puede adaptarse con su procedencia anotada en el archivo (T067).
- La CI compila y prueba; **no** mide rendimiento (D-13).
- Tauri 3 con CEF no se usa (D-10).

### 2.8 Supuestos del plan y aclaraciones que le afectan

| Aclaración (spec §12) | Qué asume el plan mientras no se resuelva |
|---|---|
| 1 Licencia | **Resuelta (2026-10-22): MIT OR Apache-2.0.** Cada crate declara `license = "MIT OR Apache-2.0"`; `LICENSE-MIT` y `LICENSE-APACHE` en la raíz (T001). `deny.toml` bloquea GPL, LGPL, AGPL y licencias desconocidas de forma permanente (T003); las bibliotecas del sistema enlazadas dinámicamente (GTK/WebKitGTK en Linux) no cuentan como dependencia de crate |
| 2 Hardware Windows/Linux, 120 Hz | **Resuelta (2026-10-22): solo el Mac por ahora** (MacBook Air M1, 60 Hz). Las tareas `[HW-…]` de `tasks.md` siguen bloqueadas; el informe marca «sin datos» lo no medido y la decisión de interfaz es provisional para Windows y Linux (`spec.md` §9) |
| 3 Versión de ONLYOFFICE en Windows/Linux | Se usa la última 10.x estable disponible y se registra la versión exacta |
| 4 Atajo de edición en macOS | Resuelta en `spec.md`: se aceptan **Ctrl+U** (el de Excel para Mac, `[VERIFICADO]` en https://exceljet.net/shortcuts/edit-the-active-cell) y **F2** |
| 5 Idioma de fórmulas | El idioma es seleccionable (`--lang es` o `--lang en`, por defecto el del sistema si está entre `en, es, fr, de, it`) **y** el núcleo acepta nombres del otro idioma al teclear (§5.7). A2 se abre con `es` |
| 6 Copiar con formato o valor | Se copia el **texto formateado tal como se ve** (lo que Excel pone en el portapapeles como texto) |
| 7 Archivo enorme | Resuelta en `spec.md`: hasta el límite de Excel (1 048 576 × 16 384) sin congelarse; abrir ocurre en un hilo de trabajo con indicador «Abriendo…»; si no cabe en memoria, mensaje claro sin cerrarse. Prueba de estrés opcional de 1 000 000 × 20 |
| 8 Contenido no soportado | Resuelta en `spec.md`: se ignora al mostrar y **se avisa al abrir** si el archivo trae partes que la PoC no conserva (`[VERIFICADO]` en T006: IronCalc 0.9.1 **no** las informa, porque `load_from_xlsx` devuelve `Result<Model, XlsxError>` sin canal de avisos y un gráfico, una imagen o una validación de datos no dan ninguna señal; por eso la detección es propia, sobre las partes del paquete). En esos archivos, Cmd/Ctrl+S **pide confirmación** antes de sobrescribir y «Guardar como» propone un nombre nuevo; en los demás, Cmd/Ctrl+S sobrescribe sin preguntar |
| 9 y 10 Regla de decisión / agregación | El agregador implementa la lectura de `spec.md` («en alguna de las dos» para ≥2× en M-03/M-04; p95 por repetición y mediana entre repeticiones) |
| 11 Umbrales | Resuelta en `spec.md`: se adoptan los propuestos y se congelan en `criteria.toml` antes de la primera corrida oficial (T049); no se cambian después de ver datos |
| 12 Archivos A4 | **Resuelta (2026-10-22):** archivos del usuario + corpus públicos (Apache POI `test-data/spreadsheet`, Apache-2.0; LibreOffice `sc/qa/unit/data/xlsx`, MPL-2.0), incluidos `.xlsm`. Un manifiesto (`testdata/a4/manifest.toml`, que sí se versiona) lista fuente, licencia y SHA-256; los archivos van a `testdata/a4/` (ignorado por git). Nunca se ejecutan macros (T114, T115). **A4 alimenta SC-007** (T115–T118) y se informa por fuente (usuario / Apache POI / LibreOffice); sin A4 preparado, SC-007 queda `sin datos` |
| 13 Alojamiento del repositorio | **Resuelta (2026-10-22): GitHub público** https://github.com/jahziel1/dokumentz (vacío, rama `main`). Ejecutores de Actions gratuitos sin límite de minutos por ser público. `git init` y el primer `push` se hacen en CP-0 con confirmación del dueño |
| 15 y 16 Arquitectura de la suite y umbrales de SC-007 | **Resueltas (2026-10-22).** Hoja de ruta y arquitectura en `specs/000-arquitectura-suite/arquitectura.md`; nombres de producto (D-18); los umbrales de SC-007 se adoptan como propuesta (aclaración 16) y se congelan en `criteria.toml` junto a los de SC-001…SC-005 (T047, T049). Decidido en 000 (§4.4): tres aplicaciones, como Office |
| Extensión de desplazamiento (nuevo, decisión del plan) | La extensión desplazable es el **rango usado** más una pantalla de holgura; crece si la selección sale de ella. «Hasta el final» en US1-5 significa la última fila con datos |

---

## 3. Verificación contra la constitución

Las preguntas de «Compuertas de la constitución» de `specs/constitucion.md` (versión 1.1.1) se responden así.

| Principio | Pregunta de la compuerta | Cómo la cumple la PoC | Evidencia / tareas |
|---|---|---|---|
| I. Libre y gratuito | ¿Alguna dependencia GPL/LGPL/AGPL, cuenta, nube o telemetría? Debe ser «no» | Licencia MIT OR Apache-2.0; `deny.toml` bloquea GPL, LGPL, AGPL y licencias desconocidas; CSP sin orígenes externos; instrumentación apagada por defecto, escribe solo JSON local; no hay pantalla de inicio de sesión. Crates internos de Zed bajo `gpui_platform`: todos Apache-2.0 `[VERIFICADO]`. La licencia del proyecto sigue en aclaración 1 | T003, T051, T012, T108, T109 (sesión con la red apagada y revisión de conexiones) |
| II. Velocidad | ¿Cada cambio de interfaz o núcleo lleva medición antes/después contra su presupuesto? | Las tareas marcadas `[MIDE]` en `tasks.md` adjuntan JSON de la instrumentación interna (`results/dev/…`). Los presupuestos están en §2.6 y en `spec.md` §6/§8 | T030, T031, T056, T061, T063, T071 |
| III. Regla de oro | ¿El núcleo compila y se prueba sin interfaz y sin depender de ella? | `dokumentz-sheet` solo depende de `ironcalc*`, `serde`, `thiserror` y (opcional, por característica) `dokumentz-metrics`. Prueba `no_ui_deps` en CI. Hit-testing, geometría, formato de números, fórmulas, archivos y TSV del portapapeles viven en el núcleo. Las interfaces dibujan y reciben entrada; la **única** excepción es EX-01 | T004, T015–T032 |
| IV. Formatos Office nativos | ¿Guardar y abrir usan el formato Office directamente y pasan la ida y vuelta? | `open_xlsx`/`save_xlsx` sin diálogo de conversión; Cmd/Ctrl+S sobrescribe el `.xlsx` abierto; ida y vuelta automática con lector independiente; jueces `x2t` (ONLYOFFICE), LibreOffice y Excel si hay; además `compat-check` mide si el motor abre los archivos reales con los resultados de Excel (SC-007, sin ejecutar macros) | T077, T078, T079, T115–T118 |
| V. Sentirse como Office, diseño propio | ¿Los atajos y nombres siguen al sistema y el diseño visual es propio? | Tabla de atajos por SO (§6.7 y §7.6), funciones en `es`/`en`. Sin cinta. Diseño visual propio (colores y tipografía de sistema, ningún recurso de Microsoft); registro de procedencia del icono de la app | T055, T060, T111 |
| VI. Multiplataforma real | ¿Se probó en macOS, Windows y Linux y el rendimiento en hardware real? | CI de 3 SO (compilar + pruebas funcionales). Rendimiento solo en hardware real; mediciones en VM se marcan «solo funcional». El informe incluye modelo, GPU, SO y versión del motor web | T081, T093–T105 |
| VII. No copiar de ONLYOFFICE | ¿Se evitó copiar código y se usó como juez? | Ningún archivo de `sdkjs`/`web-apps`/`core` en el árbol; `x2t` se invoca como programa externo y el resultado se compara con `xlsx-diff`. Lectura de revisión de procedencia en cada punto de control | T037, T078, CP-* |
| VIII. Medir antes de decidir | ¿Decisión con datos reproducibles y regla fijada antes de medir? | La regla de `spec.md` §9 está codificada y probada rama por rama en `bench-report`; `criteria.toml` (con los umbrales de SC-007) se congela con etiqueta git y su hash viaja en cada JSON (`criteria_sha`). La regla del motor (SC-007) se fija antes de medir y se aplica por separado de la de interfaz. Semilla fija. Se reportan también las medidas en que la PoC pierde | T047–T049, T107, T117 |
| IX. Etapas cortas | ¿La etapa termina con pruebas en verde y un diff revisado? | `tasks.md` agrupa en etapas de < 150k de contexto con puntos de control `CP-n` en los que el agente principal vuelve a correr las pruebas y revisa el diff | `tasks.md` |

### Excepciones registradas (Gobierno, regla 5)

| ID | Principio | Motivo | Alcance | Aprobó | Caduca |
|---|---|---|---|---|---|
| EX-01 | III (fuentes y acomodo en el núcleo) | La PoC no incluye motor de fuentes en el núcleo. Cada interfaz mide y traza texto con su propio sistema (canvas 2D `measureText` en Tauri; `shape_line` en GPUI) para recortar, desbordar y mostrar `####`. El núcleo entrega el texto formateado, el estilo y cuántas celdas vacías contiguas hay a la derecha (`spill_cols`) | Solo medición y trazado de texto de celdas de la cuadrícula. Nada de fórmulas, formato de número ni acomodo de celdas | Dueño del proyecto, 2026-10-22 | Al cierre de la fase 0 (decisión Tauri/GPUI), antes de iniciar la fase 1 |

Otras decisiones de reparto que **no** son excepciones: el búfer de edición (texto en curso y rango de composición IME) vive en la interfaz porque es entrada; se confirma al núcleo con `CommitEdit`. La correspondencia píxel→fila del desplazamiento escalado en Tauri es presentación; la correspondencia píxel→celda del ratón es `hit_test` del núcleo.

---

## 4. Estructura del repositorio

La raíz del repositorio es `dokumentz/` (https://github.com/jahziel1/dokumentz). Los nombres siguen la arquitectura de la suite (`specs/000-arquitectura-suite/arquitectura.md`, D-18): lo que construye esta PoC son **piezas de producto**; solo `apps/` contiene una pieza provisional (la interfaz que pierda se borra al cerrar la fase 0).

```
dokumentz/
├── Cargo.toml                     # [workspace], [workspace.dependencies], [patch.crates-io] de GPUI, perfiles
├── Cargo.lock
├── rust-toolchain.toml            # channel = "1.99.0"
├── .cargo/config.toml             # alias test-core = test --workspace --exclude dokumentz-app-tauri --exclude dokumentz-app-gpui
├── deny.toml                      # cargo-deny: licencias y fuentes
├── rustfmt.toml  .editorconfig  .gitignore
├── specs/
│   ├── constitucion.md
│   ├── 000-arquitectura-suite/    # arquitectura.md: capas, fases y hoja de ruta de la suite (entrada de este plan)
│   └── 001-poc-hoja-de-calculo/   # spec.md research.md plan.md tasks.md (+ decision.md al cierre)
├── crates/                        # piezas permanentes (se conservan en las fases siguientes)
│   ├── dokumentz-sheet/                 # MOTOR DE LA HOJA: IronCalc + vista de cuadrícula + comandos (sin dependencias de interfaz)
│   │   ├── src/
│   │   │   ├── lib.rs             # API pública y reexportaciones
│   │   │   ├── core.rs            # struct Core, abrir/guardar, apply(Command)
│   │   │   ├── command.rs         # Command, Dir, Advance, Hit, UiState (serde)
│   │   │   ├── geometry.rs        # aristas de filas/columnas, extensión, hit-test
│   │   │   ├── view.rs            # scroll en píxeles, ensure_visible
│   │   │   ├── drawlist.rs        # DrawList, CellStyle, construcción desde UserModel
│   │   │   ├── codec.rs           # formato binario v1 (codificar/decodificar)
│   │   │   ├── nav.rs             # navegación y selección sobre UserModel
│   │   │   ├── edit.rs            # sesión de edición, idioma de fórmulas
│   │   │   ├── clipboard.rs       # TSV de copia y pegado
│   │   │   ├── save.rs            # guardado atómico
│   │   │   ├── inspect.rs         # lectura de solo lectura de valores sin formato y fórmulas (la usa tools/compat-check, §5.9)
│   │   │   └── error.rs
│   │   ├── tests/                 # nav, edit, formulas_es_en, undo_deps, clipboard, roundtrip, no_ui_deps, spike_*
│   │   ├── tests/golden/drawlist_v1.bin
│   │   └── examples/{bench_drawlist,bench_edit}.rs
│   ├── dokumentz-metrics/               # instrumentación (FR-012): Recorder, Span, Histogram, JSON; tipos del esquema de resultados (permanente y compartida)
│   └── dokumentz-script/                # guiones de prueba compartidos: Step, Script, ScriptResult; binario dokumentz-script-check
├── tools/                         # herramientas permanentes: cada fase mide velocidad y compatibilidad
│   ├── testdata-gen/              # generador (FR-013): A1, A2, A3, empty, small; binario testdata-gen
│   ├── compat-check/              # compatibilidad con archivos reales (FR-014, M-09, SC-007): lib + bin (run, file, missing)
│   │   ├── src/{lib,main,producer,reference,compare,volatile,functions,report,supervisor,missing}.rs
│   │   └── tests/                 # fixtures generados con rust_xlsxwriter + set_formula_result (ver tests/fixtures/compat/)
│   ├── xlsx-diff/                 # comparación celda a celda con calamine (lib + bin)
│   ├── bench-runner/              # método externo: env, preflight, captura, entrada, memoria, juez, roundtrip, calibrar, run
│   ├── bench-report/              # agregación, criterios, regla de decisión de interfaz, regla del motor, informe
│   │   └── criteria.toml          # umbrales congelados (SC-001…SC-005 y SC-007)
│   ├── corpus/                    # fetch.sh: descarga de los corpus públicos de A4 con hashes (T114; requiere visto bueno del dueño)
│   └── ci/                        # listas de paquetes del SO, scripts de apoyo
├── apps/                          # interfaces candidatas (la perdedora se borra al cerrar la fase 0; la ganadora es la semilla de apps/hoja)
│   ├── tauri/
│   │   ├── package.json  vite.config.ts  tsconfig.json  index.html
│   │   ├── src/                   # TypeScript: main.ts drawlist.ts renderer.ts scroller.ts input.ts editor.ts ime.ts
│   │   │                          #             formulabar.ts tabs.ts script.ts metrics.ts   (+ *.test.ts)
│   │   └── src-tauri/             # crate dokumentz-app-tauri
│   │       ├── Cargo.toml  tauri.conf.json  capabilities/default.json
│   │       └── src/{main.rs, lib.rs, commands.rs, state.rs, cli.rs}
│   └── gpui/                      # crate dokumentz-app-gpui
│       ├── Cargo.toml
│       └── src/{main.rs, app.rs, grid.rs, scrollbar.rs, text_input.rs, editor.rs,
│                formula_bar.rs, tabs.rs, keymap.rs, script.rs, metrics.rs, cli.rs}
├── tests/
│   ├── functional/{us1,us2,us3}/  # guiones JSON compartidos por las dos apps
│   ├── fixtures/                  # xlsx pequeños versionados (≤ 100 KB); compat/ para compat-check
│   └── manual/fr007-ime.md        # guion manual de SC-004
├── docs/medicion/RUNBOOK.md       # procedimiento paso a paso por plataforma (T083)
├── docs/procedencia.md            # origen y licencia de cada recurso y de cada fragmento adaptado (T111)
├── testdata/                      # A1–A3 generados y A4 local (ignorado por git, salvo testdata/a4/manifest.toml)
├── results/<plataforma>/<run-id>/ # JSON de resultados, capturas de verificación, informe; privado/ ignorado por git
└── .github/workflows/ci.yml
```

**Qué crates de la base común llegan en la fase 1 (no se crean en esta PoC).** La arquitectura (000) prevé además `dokumentz-opc` (paquete de Office; conserva las partes que no entiende, como macros y gráficos aún no soportados), `dokumentz-drawing` (DrawingML compartido; fases 1–4), `dokumentz-text` (motor de texto común; lo decide la PoC 002 y al entrar se cierra EX-01), `dokumentz-render` (protocolo de dibujo genérico; la lista de dibujo de la cuadrícula de §5.4 se convertirá en su primer cliente), `dokumentz-edit` (comandos, deshacer/rehacer, selección y portapapeles con formatos) y `dokumentz-shell` (biblioteca de interfaz común: ventana de documento, cinta, menús y atajos, en la tecnología que gane la fase 0a), más las tres aplicaciones finales `apps/hoja`, `apps/documentos` y `apps/presentaciones`, como Office. `dokumentz-doc` nace en la PoC 002 (documentos) y `dokumentz-slide` en la fase 3. Hasta la fase 1, esas piezas comunes viven dentro de `dokumentz-sheet` (o, en el caso del texto, en cada interfaz, EX-01); su extracción a crates propios se planifica en la fase 1 según la arquitectura 000.

Reglas del árbol:

- Los miembros del espacio de trabajo se **declaran todos en T001–T002** (con código mínimo) para que las etapas siguientes no editen el `Cargo.toml` raíz en paralelo. Solo tocan los `Cargo.toml` de su propio crate.
- `dokumentz-sheet` no puede depender de `apps/*`, `tools/*` ni de `dokumentz-script`. `dokumentz-metrics` es opcional en el núcleo (característica `metrics`).
- Las herramientas de `tools/` pueden depender de `crates/*`, nunca al revés. `compat-check` depende de `dokumentz-sheet` (el motor bajo prueba) y de `calamine` (el lector **independiente**); no puede usar IronCalc para leer los valores guardados. Ninguna herramienta ejecuta macros.
- Contingencia R-10: si Tauri y GPUI no resuelven dependencias en un solo `Cargo.lock`, `apps/gpui` pasa a ser un espacio de trabajo independiente que depende de `crates/*` por ruta.


---

## 5. Diseño del núcleo (`crates/dokumentz-sheet`)

### 5.1 Límites

- Dependencias permitidas: `ironcalc`, `ironcalc_base`, `serde`, `thiserror` y, por la característica `metrics`, `dokumentz-metrics`. Nada de ventanas, async runtimes ni portapapeles del sistema (la interfaz mueve cadenas entre el portapapeles del SO y el núcleo).
- Todo lo que decide qué se ve (texto formateado, estilo resuelto, geometría, qué celda está bajo un punto, qué celda queda activa tras una tecla) lo calcula el núcleo y es probable sin ventana.
- Índices: IronCalc usa **hojas base 0 (`u32`) y filas/columnas base 1 (`i32`)**: `set_user_input(0, 1, 1, …)` escribe en A1 `[VERIFICADO]` en las pruebas de `main` (`base/src/test/user_model/test_language_switch.rs`). El núcleo expone los mismos índices.
- Dimensiones máximas del motor: 1 048 576 filas × 16 384 columnas; tamaños por defecto 90,0 de ancho y 25,0 de alto `[VERIFICADO]` en `base/src/constants.rs` de `main` (constantes `pub(crate)`; se leen con `get_column_width`/`get_row_height`). Unidad `[VERIFICADO]` en T006 (0.9.1): píxeles lógicos **de IronCalc**, no los de Excel: ancho = caracteres del archivo × 9 y alto = puntos × 1,5625 (constantes públicas `COLUMN_WIDTH_FACTOR` y `ROW_HEIGHT_FACTOR`); lo oculto mide 0,0; lo no definido mide 90×25 (Excel a 96 ppp: 64×20). A igual zoom se ven menos filas que en Excel u ONLYOFFICE; hay que tenerlo en cuenta al comparar.

### 5.2 Estado y construcción

El tiempo de vida de `UserModel<'a>` viene de las cadenas de configuración (`locale`, `timezone`, `language`) `[VERIFICADO]` (docs.rs). El núcleo usa literales `'static` (`"en"`, `"es"`, `"UTC"`), así que el modelo es `UserModel<'static>`, `Send` y `Sync` `[VERIFICADO]` (lista de «Auto Trait Implementations» de docs.rs).

```rust
// crates/dokumentz-sheet/src/core.rs  (pseudocódigo)
pub enum Lang { Es, En }                       // language_id "es"/"en" (solo nombres de función); locale_id aparte: decide separadores y decimales (§5.7; locales_list.json: en, en-GB, de, es, fr, it)

pub struct OpenOptions { pub lang: Lang, pub locale: &'static str /* "en" o "es"; ver §5.7 */, pub tz: &'static str /* "UTC" */ }

pub struct Core {
    um: UserModel<'static>,
    lang: Lang,
    path: Option<PathBuf>,
    dirty: bool,
    data_version: u64,                  // sube con todo cambio de datos o de hoja activa
    geometry: Vec<Option<SheetGeometry>>,  // perezoso, por hoja; se invalida al cambiar tamaños
    view: View,                         // hoja, scroll (x, y) en px lógicos, viewport (w, h, escala)
    block: Option<CachedBlock>,         // último DrawList codificado y su rango
    edit: Option<EditSession>,
    drawn_cells_last: u32,              // para el invariante FR-003 y la instrumentación
    rec: Option<Arc<dokumentz_metrics::Recorder>>,   // solo con la característica `metrics`
}

impl Core {
    pub fn new_empty(opts: OpenOptions) -> Result<Core, CoreError>;
    // ironcalc::import::load_from_xlsx(file_name: &str, locale: &str, tz: &str, language: &str)
    //     -> Result<Model<'a>, XlsxError>                                   [VERIFICADO] docs.rs
    // luego UserModel::from_model(model)                                    [VERIFICADO]
    pub fn open_xlsx(path: &Path, opts: OpenOptions) -> Result<Core, CoreError>;
    // ironcalc::export::save_to_xlsx(model: &Model<'_>, file_name: &str) -> Result<(), XlsxError>
    //                                                                       [VERIFICADO] docs.rs
    // se llama con self.um.get_model()  [VERIFICADO: get_model(&self) -> &Model<'_>]
    // [VERIFICADO en T006, 0.9.1] save_to_xlsx NO sobrescribe un archivo existente y PANICA si la carpeta no existe:
    //   el núcleo abre su propio File (tmp) y usa save_xlsx_to_writer; luego rename.
    pub fn save_xlsx(&mut self, path: Option<&Path>) -> Result<SaveInfo, CoreError>;   // atómico: tmp + rename

    pub fn apply(&mut self, cmd: Command) -> Result<UiState, CoreError>;     // todo lo que cambia estado
    pub fn ui_state(&self) -> UiState;
    pub fn copy_text(&self) -> String;                                        // TSV de la selección
    pub fn hit_test(&self, x: f32, y: f32) -> Hit;                            // coordenadas de la cuadrícula
    pub fn sheet_metrics(&mut self) -> SheetMetrics;                          // extensión desplazable
    pub fn draw_block(&mut self, req: DrawRequest) -> Arc<DrawList>;          // caché por (hoja, rango, data_version)
    pub fn encode(dl: &DrawList) -> Vec<u8>;                                  // formato binario v1 (§5.4)
}
```

`load_from_xlsx_bytes` (devuelve `Workbook`) y `save_xlsx_to_writer<W: Write + Seek>` también existen `[VERIFICADO]`; se evalúan en T015 para abrir/guardar sin pasar por rutas de texto (útil para guardado atómico y para pruebas en memoria).

### 5.3 Geometría y vista

```rust
pub struct SheetGeometry {
    default_w: f32, default_h: f32,
    col_overrides: Vec<(i32 /*col*/, f32 /*ancho*/)>,    // ordenado; solo las distintas del valor por defecto
    row_overrides: Vec<(i32, f32)>,
    used_rows: i32, used_cols: i32,                      // rango usado (última celda con dato)
}
impl SheetGeometry {
    fn col_x(&self, col: i32) -> f64;  fn row_y(&self, row: i32) -> f64;   // O(log k) con k = número de excepciones
    fn col_at(&self, x: f64) -> i32;   fn row_at(&self, y: f64) -> i32;    // búsqueda binaria sobre las excepciones
    fn extent(&self, view: &View) -> (f64, f64);                           // usado + una pantalla de holgura
}
```

- Se construye una vez por hoja leyendo **directamente** `get_model().workbook.worksheets[i].rows` y `.cols` (públicos, `[VERIFICADO]` en T006: `Row { r, height /*puntos*/, hidden, … }` y `Col { min, max, width /*caracteres*/, custom_width, hidden, … }`; se aplican `ROW_HEIGHT_FACTOR`/`COLUMN_WIDTH_FACTOR` y lo oculto vale 0) y guardando solo lo distinto del valor por defecto. **No** se llama a `get_row_height` fila por fila: recorre `rows` en cada llamada (lineal en el número de entradas `Row`; medido: 15 µs por llamada con 100 000 entradas, ≈ 1,5 s para 100 000 llamadas), mientras que leer `rows` entero cuesta 0,14 ms. El presupuesto de T016 (≤ 20 ms) se mide sobre la lectura directa.
- Constantes de cabecera comunes a ambas UI y definidas en el núcleo: `ROW_HEADER_W = 46`, `COL_HEADER_H = 25` (px lógicos, propuesta). Así el recorte y el hit-test son idénticos.
- `View` guarda el desplazamiento en **píxeles lógicos de hoja** (no en filas). `ScrollTo/ScrollBy` lo acotan a `[0, extent − viewport]`. Tras cualquier navegación, `ensure_visible(active)` ajusta el desplazamiento y llama a `set_top_left_visible_cell` para que `UserModel` coincida; `set_window_width/height` se llaman al cambiar el viewport (necesarios para `on_page_up/down`) `[VERIFICADO]` que existen; `[VERIFICADO]` en T022 cómo reconcilia `UserModel` su desplazamiento propio (`get_scroll_x/y`) con el nuestro (T006 adelanta: `UserModel` solo guarda la celda superior izquierda visible; `get_scroll_x/y` se deducen sumando los tamaños de las filas/columnas anteriores, en O(n) por llamada, así que no hay desplazamiento por píxel y el nuestro se lleva aparte).

### 5.4 Lista de dibujo y formato binario v1

```rust
pub struct DrawRequest { sheet: u32, x0: f64, y0: f64, w: f32, h: f32, margin_x: f32, margin_y: f32 }

pub struct DrawList {
    sheet: u32, data_version: u64,
    first_row: i32, first_col: i32, n_rows: u32, n_cols: u32,
    origin: (f64, f64),                 // px de hoja de la esquina superior izquierda del bloque
    col_edges: Vec<f32>, row_edges: Vec<f32>,    // n+1 aristas relativas al origen
    col_labels: Vec<StrRef>, row_labels: Vec<StrRef>,   // «A», «B»… y «1», «2»… (el núcleo los calcula)
    styles: Vec<CellStyle>,             // deduplicados
    cells: Vec<DrawCell>,               // dispersa: solo celdas con texto o con relleno
    strings: Vec<u8>,                   // UTF-8 concatenado
}
pub struct CellStyle {                  // ya resuelto: nada de temas ni «General» sin resolver
    fg: Rgba, fill: Option<Rgba>, font_px: f32, family: u16,
    bold: bool, italic: bool, underline: bool, strike: bool, wrap: bool,
    h_align: Align /*Izq, Centro, Der*/, v_align: VAlign,
}
pub struct DrawCell { row_off: u32, col_off: u32, text: StrRef, style: u16, kind: Kind /*Texto, Número, Bool, Error, SoloEstilo*/, spill_cols: u8 }
```

Origen de cada dato `[VERIFICADO]` (docs.rs): texto con formato de número = `get_formatted_cell_value(sheet,row,col) -> Result<String,String>`; estilo = `get_cell_style(..) -> Result<Style,String>` con `Style { alignment: Option<Alignment>, num_fmt: String, fill: Fill, font: Font, border: Border, quote_prefix: bool }`, `Font { strike, u, b, i, sz: i32, color: Color, name, family, scheme }`, `Fill { color: Color }`, `Alignment { horizontal, vertical, wrap_text }` y `Color { Rgb(String), Theme(i32, f64), None }`; color resuelto = `resolve_color(&Color) -> String`; tipo = `get_cell_type(..) -> CellType`. Alineación `General`: número a la derecha, texto a la izquierda, booleano y error al centro (como Office). `spill_cols` = columnas vacías contiguas a la derecha (tope 8) para desbordar texto izquierdo; para números que no caben la interfaz dibuja `####` (EX-01).

**Formato binario v1** (little-endian, secciones alineadas a 4 bytes). Es el contrato entre `codec.rs` y `drawlist.ts`; se congela con el archivo dorado `tests/golden/drawlist_v1.bin` (T018).

| Sección | Contenido |
|---|---|
| Cabecera (64 B) | `u32` magia `0x31444C50` («PDL1»), `u16` versión = 1, `u16` banderas, `u64` `data_version`, `u32` hoja, `i32` `first_row`, `i32` `first_col`, `u32` `n_rows`, `u32` `n_cols`, `u32` `n_styles`, `u32` `n_cells`, `u32` `strings_len`, `f64` `origin_x`, `f64` `origin_y` |
| Aristas de columnas | `f32 × (n_cols + 1)`, relativas a `origin_x` |
| Aristas de filas | `f32 × (n_rows + 1)`, relativas a `origin_y` |
| Etiquetas | `n_cols` y luego `n_rows` pares `(u32 str_off, u32 str_len)` |
| Estilos | `n_styles × 20 B`: `u32 fg`, `u32 fill` (0 = sin relleno), `f32 font_px`, `u16 family`, `u8 flags` (negrita=1, cursiva=2, subrayado=4, tachado=8, ajuste=16), `u8 h_align`, `u8 v_align`, `u8[3]` reservado |
| Celdas | `n_cells × 20 B`: `u32 row_off`, `u32 col_off`, `u32 str_off`, `u32 str_len`, `u16 style`, `u8 kind`, `u8 spill_cols` |
| Cadenas | `strings_len` bytes UTF-8 |

Tamaño típico: una pantalla de 1440×900 con margen de ±1 altura y ±½ ancho son unas 2 800 celdas ≈ 100 KB.

**Caché de bloque en el núcleo.** `draw_block` devuelve el bloque anterior (`Arc`) si el área pedida cabe en él y `data_version` no cambió. En GPUI, así, pintar un cuadro es leer un `Arc`; en Tauri, el mismo `Arc` se codifica una sola vez por bloque.

### 5.5 Comandos y estado de la interfaz

```rust
pub enum Command {
    // vista
    SetViewport { w: f32, h: f32, scale: f32 },
    ScrollTo { x: f64, y: f64 },  ScrollBy { dx: f64, dy: f64 },
    // selección y navegación (FR-004)
    Click { x: f32, y: f32, extend: bool },  DragTo { x: f32, y: f32 },
    Move { dir: Dir, extend: bool },         // flechas; Mayús = extend
    Edge { dir: Dir, extend: bool },         // Ctrl/Cmd+flecha
    Page { down: bool, extend: bool },       // RePág/AvPág
    Home { ctrl: bool },                     // Inicio / Ctrl+Inicio
    SelectSheet(u32),
    // edición (FR-005)
    BeginEdit { mode: EditMode /*Reemplazar, Anexar, BarraDeFórmulas*/, seed: Option<String> },
    SetEditText(String),                     // opcional, no bloquea la UI: resaltado/diagnóstico
    CommitEdit { text: String, advance: Advance /*Abajo, Arriba, Derecha, Izquierda, Ninguno*/ },
    CancelEdit,
    // historial (FR-008)
    Undo, Redo,
    // portapapeles (FR-010) y archivo (FR-011)
    PasteText(String),
    Save { path: Option<PathBuf> },
    SetLang(Lang),
}

pub struct UiState {                          // pequeño; viaja como JSON
    data_version: u64, sheet: u32,
    active: (i32, i32), range: (i32, i32, i32, i32),
    scroll: (f64, f64), extent: (f64, f64),
    edit: Option<EditState /*celda, texto inicial*/>,
    formula_bar: String,                      // contenido de la celda activa (get_cell_content)
    sheets: Vec<SheetTab /*nombre, color, activa*/>,
    can_undo: bool, can_redo: bool, dirty: bool,
    busy: Option<String>, message: Option<String>,
}
```

### 5.6 Selección, navegación y edición sobre `UserModel`

| Acción | Llamada de `UserModel` `[VERIFICADO]` que existe | Notas |
|---|---|---|
| Flechas | `on_arrow_right/left/up/down` | `[VERIFICADO]` en T021: saltan las filas y columnas ocultas (prueba `flechas_saltan_las_filas_y_columnas_ocultas`) |
| Mayús+flecha | `on_expand_selected_range(key: &str)` | `key` = `KeyboardEvent.key` del DOM: `"ArrowRight"`, `"ArrowLeft"`, `"ArrowUp"`, `"ArrowDown"`; cualquier otra cadena se ignora **sin error** `[VERIFICADO]` en T006; `range` = `[fila_ini, col_ini, fila_fin, col_fin]` normalizado y el ancla no se mueve |
| Clic | `set_selected_cell(row, column)` | tras `hit_test` del núcleo |
| Arrastre | `on_area_selecting(target_row, target_column)` | |
| Ctrl/Cmd+flecha | `on_navigate_to_edge_in_direction(NavigationDirection::{Left,Right,Up,Down})` | extender con Mayús es opcional: no hay API; se haría con `get_last_non_empty_in_row_before_column` y `get_first_non_empty_in_row_after_column` (solo filas) |
| RePág/AvPág | `on_page_up/down` | dependen de `set_window_height` |
| Hoja activa | `set_selected_sheet(u32)`, `get_selected_sheet`, `get_worksheets_properties` | |
| Valor para la barra de fórmulas | `get_cell_content(sheet,row,col)` | en el idioma activo |
| Confirmar | `set_user_input(sheet,row,col,&str)` | evalúa **todo el libro** salvo `pause_evaluation()` (R-04) |
| Deshacer/rehacer | `undo()`, `redo()`, `can_undo()`, `can_redo()` | |
| Selección actual | `get_selected_view() -> SelectedView`, `get_selected_cell() -> (u32,i32,i32)` | `SelectedView { sheet, row, column, range: [i32;4], top_row, left_column }` `[VERIFICADO]` en T006: `row`/`column` son el ancla, no expone el foco (se deduce de `range`) y el tipo no se puede nombrar fuera de IronCalc (`pub use` bajo `#[cfg(test)]`), solo usarlo por inferencia |

`EditSession` guarda la celda, el modo y el texto original (para Esc). La regla de avance de Office se implementa en el núcleo: Intro baja (Mayús+Intro sube), Tab va a la derecha (Mayús+Tab a la izquierda); si hay un rango seleccionado de varias celdas, el avance recorre el rango (`[NO VERIFICADO]` en ONLYOFFICE; se comprueba en T023 y, si difiere, se deja como celda única).

### 5.7 Idioma de fórmulas (FR-006)

Hechos `[VERIFICADO]` en `main` (`base/src/test/test_language.rs`): `Model::new_empty("model","en","UTC","de")` crea un modelo con idioma alemán; tras `set_language("es")`, una fórmula escrita como `=TRUE()` da `#¿NOMBRE?`, mientras que la fórmula ya guardada se reescribe como `=VERDADERO()`: **se guarda en inglés y se muestra en el idioma activo**. Un archivo `.xlsx` trae las fórmulas en inglés, así que A2 se ve en español con solo abrirlo con `Lang::Es`.

Política de teclear (FR-006, aclaración 5):

1. `CommitEdit` con texto que empieza por `=` se aplica en el idioma activo con `set_user_input`.
2. Si la celda queda con el error de nombre desconocido (`#¿NOMBRE?` en `es`, `#NAME?` en `en`), el núcleo **reintenta con el otro idioma**: `undo()` (deshace el primer intento y lo saca del historial activo), `set_language(otro)`, `set_user_input` con el mismo texto, `set_language(activo)`. El historial neto queda con **una** entrada de edición.
3. Si el reintento tampoco da un valor válido, se vuelve al resultado del primer intento (para que el texto tecleado no se pierda y se pueda corregir).

Hechos que ayudan `[VERIFICADO]` en el código de `main`: `UserModel::set_language` delega en `Model::set_language`, que solo cambia el analizador y el idioma (`self.parser.set_language(language); self.language = language;`): **no evalúa y no entra en el historial** (`base/src/user_model/common.rs` y `base/src/model.rs`). `[VERIFICADO]` contra 0.9.1 en T006 (con la evaluación pausada, una fórmula sin evaluar sigue sin evaluar tras `set_language`; y no deja entrada en el historial). `[VERIFICADO]` en T024 (se eligió **(B)**, ver abajo) el coste del reintento sobre A2: cada `set_user_input` evalúa todo el libro (R-04), así que un intento fallido más el reintento cuestan tres evaluaciones completas. Medido en T006 (100 000×20 y 10 000 fórmulas, ≈ 38 ms por evaluación): el reintento (A) ingenuo cuesta ≈ 2,9× una edición normal y, con `pause_evaluation` (una evaluación para descubrir el error y otra al final), ≈ 1,9×, o sea que la regla de abajo (≤ 2×) elegiría (A) con pausa; T024 lo confirma sobre A2.

**Locale ≠ idioma `[VERIFICADO]` en T006 (0.9.1).** El idioma solo decide los nombres de función; el `locale` decide el separador de argumentos y el decimal al teclear. Con `locale` `es`: argumentos con `;` y decimal `,` (`=SUMA(1,2)` se lee como `SUMA(1.2)`, y `1.5` queda como texto). Con `locale` `en` e idioma `es`: `,` separa argumentos y `.` es el decimal (como Excel en español de México). Suponer «`locale_id` igual al idioma» fija el convenio de España, así que el núcleo expone `locale` aparte. **Decisión del dueño (CP-0, 2026-10-22): el `locale` sigue a la región del sistema, como en Excel.** La interfaz lee la región del SO (en macOS, `AppleLocale`, por ejemplo `es_MX`) y la pasa al núcleo, que elige el `locale` de IronCalc según el convenio numérico de esa región: si la región usa coma decimal (`es_ES`, `es_AR`, `de_DE`…) elige `es`/`de`/`fr`/`it`; si usa punto decimal (`es_MX`, `es_US`, `en_US`…) elige `en`. El mapeo es una tabla del núcleo con pruebas (T015/T024), y `--locale` en la línea de comandos lo fuerza para guiones y mediciones. En el Mac del proyecto (`es_MX`) da `locale` `en` con idioma `es`: `=SUMA(1,2)` y punto decimal. Los `.xlsx` no se ven afectados: guardan la fórmula en inglés.

**Dos implementaciones candidatas; T024 elige por medición.** (A) reintento con `undo` como arriba, que es lo más simple. (B) **traducir** en el núcleo los nombres de función del idioma contrario antes de llamar a `set_user_input`, con una tabla de nombres derivada de `generate_language/src/languages.json` (datos de IronCalc, Apache-2.0), que cuesta una sola evaluación. Se elige (A) si teclear una fórmula en inglés en una sesión `es` sobre A2 cuesta ≤ 2× una edición normal; si no, (B). **Resultado de T024 (E08, `results/dev/T024.json`): (B).** Edición normal ≈ 34,5 ms; (A) ≈ 3,1×; (A) con pausa ≈ 2,0× (en el límite) y además rompe Rehacer (la entrada de historial guarda el texto del otro idioma y `redo` lo vuelve a analizar en el idioma activo); (B) ≈ 1,0×. (B) no usa tabla propia: un modelo de borrador del motor decide si el texto falla en el idioma activo y se entiende en el otro, y lo reescribe con `get_cell_content` tras `set_language`. Conjunto fijo de pruebas en T024: `SUMA/SUM`, `BUSCARV/VLOOKUP`, `BUSCARX/XLOOKUP`, `SI.ERROR/IFERROR`, `CONTAR.SI/COUNTIF`, `SI/IF`, más una función inexistente (debe quedar con error y conservar el texto).

### 5.8 Portapapeles (FR-010)

- **Copiar:** el núcleo construye el TSV recorriendo el rango seleccionado con `get_formatted_cell_value` (texto tal como se ve; aclaración 6). Comillas dobles alrededor de campos con tabulador, salto de línea o comillas, duplicando las comillas, y filas separadas por `\n`. No se usa `copy_to_clipboard()` de IronCalc porque el campo `csv` de `Clipboard` es `pub(crate)` `[VERIFICADO]` (`base/src/user_model/clipboard.rs` en `main`).
- **Pegar:** `paste_csv_string(&Area, &str)`, que lee con delimitador tabulador (`ReaderBuilder::delimiter(b'\t')`), arma **una sola lista de cambios** (una entrada de historial) y evalúa una vez al final `[VERIFICADO]` en `base/src/user_model/clipboard.rs` de `main`; `[VERIFICADO]` en T006 (0.9.1): acepta campos entre comillas con tabuladores, saltos de línea y comillas duplicadas, y también `\r\n`; pero (a) **descarta en silencio las filas con distinto número de campos que la primera** (las siguientes suben un lugar) y (b) los campos vacíos finales **borran** las celdas de la derecha. Por eso, antes de pegar, el núcleo arma un rectángulo (rellena con tabuladores las filas cortas) y recorta las columnas finales vacías de todo el rectángulo (caso límite de `spec.md` §4).
- La interfaz solo mueve la cadena entre el portapapeles del SO y `copy_text()` / `PasteText`.

### 5.9 Archivos (FR-001, FR-011)

- `open_xlsx` devuelve `CoreError::{NoExiste, SinPermiso, NoEsXlsx(String), …}`; nunca entra en pánico con archivos truncados (prueba con fixtures corruptos en T015). Con `--open` por línea de comandos, un error termina el proceso con código distinto de cero y un mensaje en la salida de error (señal verificable para la automatización; `spec.md` §4).
- **Lectura de solo lectura para `tools/compat-check` (FR-014).** `compat-check` abre los archivos por el mismo camino que la aplicación (`Core::open_xlsx`, idioma `en`, zona `UTC`) y necesita, sin ventana ni edición: la lista de hojas y su rango usado, el valor **sin formato** y tipado de cada celda (número, texto, booleano, error, vacío), el texto de la fórmula si la hay, y una llamada explícita a la evaluación completa. Esa API vive en `crates/dokumentz-sheet/src/inspect.rs` (`sheet_names`, `used_range`, `cell_raw`, `cell_formula`, `recalculate`) y solo lee. `[A VERIFICAR EN T115]` qué parte la da `UserModel`/`Model` de IronCalc 0.9.1 (`get_cell_content`, `get_cell_type`, `get_formatted_cell_value`; el valor sin formato puede requerir `get_model()`); la comprobación de fondo es que tras abrir y recalcular el valor leído proviene del **recálculo** y no del valor guardado en el archivo.
- `save_xlsx`: abre `ruta.tmp-<pid>` con `File::create` propio, llama a `save_xlsx_to_writer` y renombra (`save_to_xlsx` se niega a escribir sobre un archivo existente y entra en pánico si la carpeta no existe, `[VERIFICADO]` en T006); el nombre propio de Office («Guardar») sobrescribe el archivo abierto; «Guardar como» pide ruta. Aviso de una línea si el archivo traía partes que IronCalc no modela (aclaración 8): `[VERIFICADO]` en T006 que IronCalc no las informa, así que se detectan por las entradas del paquete (`xl/charts/`, `xl/drawings/`, `xl/media/`, `xl/pivotTables/`, `xl/vbaProject.bin`) y por `<dataValidations` en las hojas, y que **las pierde al guardar**, incluidas las notas (`xl/comments1.xml`): se leen al modelo pero no se escriben.

### 5.10 Instrumentación (FR-012) y modo guion

- `dokumentz-metrics::Recorder` está **apagado salvo** `--metrics-out <ruta>` o la variable `DOKUMENTZ_METRICS_OUT`. Apagado: no reserva memoria, no abre archivos y no mide. Encendido: escribe un único JSON local al salir o al terminar el guion. No hay dependencias de red (`cargo tree` lo comprueba, T013).
- Mide: marcas (`launch`, `window_ready`, `open_request`, `first_data_frame`), duraciones por tramo (carga, `draw_block`, codificación, IPC en Tauri, pintado), contadores (`cells_in_list`, `cells_drawn_per_frame`, `placeholder_frames`) y muestras por cuadro/tecla.
- **Modo guion:** `--script <guion.json> [--script-out <resultado.json>] [--exit-after-script]`. El guion es una lista de pasos de `dokumentz-script`: `open`, `key`, `text`, `scroll`, `drag`, `wait_ms`, `wait_idle`, `mark`, `assert_cell`, `assert_selection`, `assert_formula_bar`, `save`, `quit`. Cada interfaz lo ejecuta llamando a **los mismos manejadores** que la entrada real (no al núcleo directamente), así el camino de la interfaz queda cubierto y el mismo guion sirve para CI, para pruebas funcionales y para la instrumentación interna. El resultado (`ScriptResult`) se escribe en JSON y `dokumentz-script-check` lo compara con lo esperado.

### 5.11 Concurrencia

`Core` se guarda en `Mutex<Core>`. Abrir, guardar y `CommitEdit` en A2 pueden tardar; se ejecutan en un hilo de trabajo (Tauri: `spawn_blocking` dentro de un comando `async`; GPUI: el ejecutor de fondo). Mientras una operación larga tiene el candado, **no** se pide nada al núcleo para desplazar si el bloque en caché cubre el área; las teclas pulsadas durante un recálculo se encolan en la interfaz y se reproducen en orden al volver el `UiState` (caso límite de `spec.md` §4). La interfaz muestra `UiState.busy`. Mejora posible si M-05 sale mal: evaluar en un hilo y mostrar valores previos con aviso; no entra en la PoC.

---

## 6. Diseño de la interfaz Tauri (`apps/tauri`)

### 6.1 Procesos y hilos

Un proceso principal (Rust: `Core` + runtime de Tauri) y los procesos de la webview del sistema (macOS: servicios XPC `com.apple.WebKit.WebContent/GPU/Networking`, que **no** cuelgan del proceso de la app; Windows: `msedgewebview2.exe`; Linux: `WebKitWebProcess` y `WebKitNetworkProcess`). Esto condiciona la medición de memoria (§9.5). Los comandos sin `async` se ejecutan en el **hilo principal** (el de la interfaz) `[VERIFICADO]` (v2.tauri.app/develop/calling-rust/): **todos los comandos de la PoC son `async`** y el trabajo pesado va a `spawn_blocking`.

### 6.2 Contrato del puente

`invoke` envía un `fetch` POST al protocolo personalizado `ipc` y, si la respuesta no es JSON ni texto, la lee con `response.arrayBuffer()`; si el protocolo falla (por ejemplo, CSP) cae a `postMessage` y avisa en consola `[VERIFICADO]` en `crates/tauri/scripts/ipc-protocol.js` (rama `dev`). `tauri::ipc::Response::new(Vec<u8>)` devuelve bytes sin pasar por JSON `[VERIFICADO]` (docs.rs y v2.tauri.app/develop/calling-rust/); el valor llega como `ArrayBuffer` en JavaScript según ese código `[VERIFICADO]`, aunque la guía no lo dice con esas palabras. `Channel<InvokeResponseBody>` permite empujar `Raw(Vec<u8>)` en orden `[VERIFICADO]` (docs.rs; la clase `Channel` de `packages/api/src/core.ts` conserva el orden por índice). **Un `Channel<Vec<u8>>` serializa a JSON y no se usa** `[VERIFICADO]`.

| Comando (todos `async`) | Entrada | Salida | Cuándo ocurre |
|---|---|---|---|
| `open_xlsx` | `{ path, lang }` | `UiState` (JSON) | abrir (diálogo o `--open`) |
| `draw_block` | `{ sheet, x0, y0, w, h, mx, my }` | `Response` binaria (DrawList v1) | fallo de caché o precarga |
| `exec` | `Command` (JSON) | `UiState` (JSON) | entrada del usuario (≤ ~100 por segundo) |
| `copy_text` | — | `String` | Cmd/Ctrl+C |
| `metrics_flush` | — | — | fin de guion |
| `ipc_echo` | `n` | `Response` de `n` bytes | solo el spike T009 y un banco opcional |

Un `Channel` de eventos `events` (JSON pequeño) lleva avisos Núcleo → JS: `opened`, `dataChanged`, `busy`, `error`. Con él, tras un `CommitEdit` el núcleo puede empujar el bloque actualizado sin esperar a que JS lo pida (decisión tras medir M-05 en T071).

**Compuerta CG-2 (T009):** si la ida y vuelta de `ipc_echo` de 128 KB tiene p95 > 10 ms en el Mac, se rediseña el reparto de bloques (bloques mayores, menos peticiones) antes de seguir; no cambia la arquitectura, que ya evita el puente en cada cuadro.

### 6.3 Caché de bloques y precarga (`scroller.ts`)

```ts
interface Block { dl: DrawList; x0: number; y0: number; x1: number; y1: number; version: number }

class BlockCache {                       // máx. 3 bloques; LRU
  lookup(vp: Viewport): Block | null     // el bloque debe CONTENER vp completo
  request(vp: Viewport, vel: Vec2): void // un solo vuelo por dirección; descarta respuestas con generación vieja
}
// pintar un cuadro nunca llama a invoke():
function frame() {
  if (!dirty) return;
  const b = cache.lookup(viewport);
  b ? renderer.paint(b, viewport, selection) : renderer.paintPlaceholder(viewport); // cuenta placeholder_frames
  dirty = false;
}
scroller.onscroll = () => { viewport.set(scrollTop * scaleY, scrollLeft * scaleX); dirty = true; maybePrefetch(); };
```

- Bloque pedido: el área visible ± 1 altura vertical y ± ½ ancho horizontal, centrada en la posición **predicha** (posición + velocidad × 150 ms). Se pide uno nuevo cuando el borde del área visible llega a menos del 25 % del borde del bloque en la dirección del desplazamiento.
- `generation` = (`data_version`, hoja, `viewport.scale`): si cambia, se descartan los bloques y se pide el visible de inmediato (único viaje; el resto se precarga).
- Salto de la barra (arrastre largo): se pinta el marco vacío con cabeceras (`paintPlaceholder`) hasta que llega el bloque; `placeholder_frames` queda registrado y se espera 0 con rueda/trackpad.

### 6.4 Desplazamiento virtual escalado

```
#viewport (overflow:auto; overscroll-behavior:none; barras con ::-webkit-scrollbar siempre visibles, 14 px)
  └─ #spacer  { width: extentW / scaleX, height: extentH / scaleY }
#grid (canvas)  position:absolute; pointer-events:none; mismo tamaño que #viewport
#overlay (canvas) selección y celda activa (no se redibuja con el desplazamiento de datos)
textarea#ime  (§6.6)
```

`scaleY = max(1, extentH / MAX_DOM_PX)` con `MAX_DOM_PX = 8 000 000` (propuesta; los límites reales de altura de elemento difieren por motor y no se verificaron aquí `[NO VERIFICADO]`; la fuente que justifica escalar está en D-09). Con A1 (100 000 filas × 25 px = 2,5 M px) `scaleY = 1`. Una prueba de T054 usa una extensión sintética de 1 000 000 filas para el caso escalado. `::-webkit-scrollbar` está soportado por los tres motores `[NO VERIFICADO]`; si alguno no lo respetara, se dibujan barras propias en el canvas.

### 6.5 Dibujo en canvas 2D (`renderer.ts`)

- Un `<canvas>` de `ancho_css × dpr` por `alto_css × dpr`; `setTransform(dpr, 0, 0, dpr, 0, 0)`. Cada cuadro: relleno de celdas (agrupado por color), cuadrícula con líneas de 1 px físico alineadas a 0,5, cabeceras fijas, texto.
- Texto: `ctx.font` se arma desde `CellStyle` (cursiva, negrita, `font_px`, familia del sistema); `measureText` con caché por `(texto, estilo)`; solo se llama a `clip()` cuando el texto no cabe en la celda más `spill_cols`; los números que no caben se pintan como `####` (EX-01).
- Selección y celda activa en `#overlay`; al cambiar solo la selección no se repinta `#grid`.
- Si en M-03 el canvas 2D no llega a los presupuestos de §2.6, la alternativa documentada es WebGL (D-09); no se adelanta.

### 6.6 Edición, textarea oculto e IME (`editor.ts`, `ime.ts`)

EditContext no existe en Safari ni Firefox `[VERIFICADO]` (D-07/caniuse), por lo que la entrada se apoya en un `<textarea id="ime">` posicionado sobre la celda activa (opacidad 0 cuando no se edita, para que el cuadro de candidatos del IME aparezca junto a la celda).

- **Escribir sobre la celda:** la primera tecla (o `compositionstart`) llega al textarea enfocado; la interfaz entra en edición con ese texto de semilla (`BeginEdit{ seed }`) y el textarea pasa a ser **el editor visible** con el estilo de la celda. El texto en curso vive en el textarea (la interfaz lo muestra al instante, sin viaje al núcleo: M-04 mide un camino local) y se refleja en la barra de fórmulas (otro `<input>`) en cada evento `input`. `SetEditText` es opcional y asíncrono.
- **Confirmar/cancelar:** Intro/Tab/Mayús → `CommitEdit{ text, advance }`; Esc → `CancelEdit`. Las teclas pulsadas mientras el comando está en curso se encolan y se reproducen en orden.
- **Máquina de estados IME** (`ime.ts`, función pura probada con Vitest): `Idle → Composing → JustEnded(t)`. Reglas: `keydown` con `isComposing` o `keyCode 229` nunca es navegación; Intro tras `compositionend` en el mismo turno del bucle de eventos se traga una vez (Safari emite `compositionend` antes que el `keydown` de Intro `[NO VERIFICADO]`; se confirma a mano en T072); clic en otra celda durante composición primero confirma la composición y luego aplica la edición; si el desplazamiento saca la celda del área visible, el textarea se mantiene enfocado y se ancla al borde visible para no perder la composición.
- **Dictado de macOS** entra como eventos `input` del textarea `[NO VERIFICADO]` (T072).

### 6.7 Atajos

| Acción | macOS | Windows / Linux |
|---|---|---|
| Abrir / Guardar / Guardar como | Cmd+O / Cmd+S / Cmd+Mayús+S | Ctrl+O / Ctrl+S / Ctrl+Mayús+S |
| Copiar / Pegar | Cmd+C / Cmd+V | Ctrl+C / Ctrl+V |
| Deshacer / Rehacer | Cmd+Z / Cmd+Mayús+Z | Ctrl+Z / Ctrl+Y (también Ctrl+Mayús+Z) |
| Editar celda | F2 y Ctrl+U (aclaración 4) | F2 |
| Ir al borde | Cmd+flecha | Ctrl+flecha |
| Extender selección | Mayús+flecha | Mayús+flecha |
| Una pantalla | Fn+↑/↓ (RePág/AvPág) | RePág / AvPág |
| Confirmar / Cancelar | Intro, Tab, Mayús+Intro, Mayús+Tab / Esc | ídem |

El módulo `mod = isMac ? e.metaKey : e.ctrlKey` decide Cmd o Ctrl. La lista se contrasta con Excel/ONLYOFFICE en cada plataforma (principio V).

### 6.8 Portapapeles, diálogos y seguridad

- Portapapeles: `tauri-plugin-clipboard-manager` (`writeText`, `readText`) `[VERIFICADO]`; el pegado desde otra app llega como texto y se envía a `PasteText`. Alternativa si el plugin diera problemas de permisos: eventos `copy`/`paste` del navegador sobre el textarea.
- Diálogos: `tauri-plugin-dialog` (`open()`/`save()` en JS o `app.dialog().file()` en Rust) `[VERIFICADO]`. Permisos mínimos en `capabilities/default.json`: `core:default`, `dialog:allow-open`, `dialog:allow-save`, `clipboard-manager:allow-read-text`, `clipboard-manager:allow-write-text`.
- CSP sin orígenes externos (T051). `[A VERIFICAR EN T051]` que el protocolo `ipc` sigue funcionando bajo esa CSP (sin el aviso de caída a `postMessage`).

### 6.9 Instrumentación interna

`metrics.ts` registra con `performance.now()`: tiempo de cada `invoke` (ida y vuelta), decodificación, pintado y tiempo entre `requestAnimationFrame` consecutivos durante el desplazamiento. Latencia de tecla = `keydown`/`beforeinput` → siguiente `requestAnimationFrame` posterior al cambio de DOM (aproximación a «presentado»). Todo se envía a Rust con `metrics_flush` y lo escribe `dokumentz-metrics`.

---

## 7. Diseño de la interfaz GPUI (`apps/gpui`)

### 7.1 Estructura

Un solo proceso. `gpui_platform::application().run(|cx| …)` abre la ventana con una vista raíz `AppView` que contiene: barra de fórmulas (`TextInput`), `GridView` (cuadrícula + barras), pestañas de hojas. El estado (`Core`, bloque actual, edición) vive en una `Entity<AppState>`. API de arranque `[VERIFICADO]` en el README de `gpui` y en los ejemplos (`gpui_platform::application()`, `cx.open_window(WindowOptions{…}, |_, cx| cx.new(…))`).

### 7.2 `GridElement`: elemento de dibujo propio

La cuadrícula **no** usa `uniform_list` ni un `div` por celda: eso recrearía un árbol de elementos por celda, lo mismo que se evita con el canvas en Tauri. `uniform_list` queda como alternativa descartada (existe y virtualiza filas `[VERIFICADO]` en `examples/uniform_list.rs`). Se implementa el trait `Element` como hace el ejemplo `input.rs`:

```rust
impl Element for GridElement {
    type RequestLayoutState = (); type PrepaintState = GridPaint;
    fn request_layout(..) { /* estilo con ancho y alto relativos a 1 → window.request_layout(style, [], cx) */ }
    fn prepaint(&mut self, .., bounds, ..) -> GridPaint {
        // 1) core.draw_block(viewport) → Arc<DrawList> (caché del núcleo: lectura barata)
        // 2) por cada celda con texto: ShapedLine desde la caché (texto, estilo) o window.text_system().shape_line(..)
        // 3) rectángulos de relleno, líneas, selección → Vec<PaintQuad>
    }
    fn paint(&mut self, .., prepaint, window, cx) {
        window.with_content_mask(Some(ContentMask{ bounds }), |window| {
            for q in prepaint.quads { window.paint_quad(q) }            // fill(bounds, color)
            for t in prepaint.lines { t.line.paint(origin, line_height, TextAlign::Left, None, window, cx).ok(); }
        });
    }
}
```

`[VERIFICADO]` en `crates/gpui/examples/input.rs` y `crates/gpui/src/window.rs` (rama `main`): `Element` con `request_layout/prepaint/paint`, `window.request_layout`, `window.text_system().shape_line(text, font_size, &runs, None)`, `ShapedLine::paint(origin, line_height, TextAlign, None, window, cx)`, `window.paint_quad(fill(bounds, color))`, `window.with_content_mask(..)`, `window.handle_input(&focus_handle, ElementInputHandler::new(bounds, entity), cx)`, `window.scale_factor()`, `window.viewport_size()`, `window.refresh_interval(cx)`, `window.request_animation_frame()`. `[A VERIFICAR EN T058]`: la firma exacta de `ContentMask`, el coste de `shape_line` por cuadro con ~1 500 celdas y si la caché interna de diseños de línea de GPUI basta o hace falta la propia. Presupuesto: ≤ 6 ms de `prepaint`+`paint` por cuadro.

### 7.3 Texto, fuentes y color

Familias por SO: macOS «Helvetica Neue» (o la del sistema), Windows «Segoe UI», Linux «Noto Sans» con alternativa «DejaVu Sans» (Tauri usa `system-ui`). Las fuentes del sistema entran por `font-kit` en macOS, DirectWrite en Windows y el sistema de texto de Linux `[VERIFICADO]` (README de `gpui`: «Without `font-kit` … no glyphs» en macOS). El ejemplo carga fuentes con un módulo de apoyo; la PoC decide en T057 si necesita empaquetar una fuente propia (si lo hace, procedencia y licencia en T111).

### 7.4 Desplazamiento

- Rueda y trackpad: `div().on_scroll_wheel(|ev, window, cx| …)`; `ScrollDelta::{Pixels(Point<Pixels>), Lines(Point<f32>)}` con `pixel_delta(line_height)` `[VERIFICADO]` (`crates/gpui/src/interactive.rs`). Se traduce a `Command::ScrollBy` en px lógicos.
- Barras: `ScrollbarElement` propio (pista y pulgar con `paint_quad`) con arrastre; la posición proporcional usa `SheetMetrics`/`UiState.extent`. Siempre visibles (misma geometría que las de Tauri: 14 px).
- Ratón: `on_mouse_down/up/move` `[VERIFICADO]` en `div.rs`; coordenadas relativas a la cuadrícula → `Command::Click/DragTo` (el núcleo resuelve la celda).

### 7.5 Entrada de texto e IME (`text_input.rs`, `editor.rs`)

GPUI delega el IME del sistema en el trait `EntityInputHandler` `[VERIFICADO]` (`examples/input.rs`): `text_for_range`, `selected_text_range`, `set_selected_text_range`, `text_length_utf16`, `marked_text_range`, `unmark_text`, `replace_text_in_range`, `replace_and_mark_text_in_range`, `bounds_for_range`, `selection_bounds_for_range`, `character_index_for_point`. El elemento registra el manejador al pintar con `window.handle_input(&focus_handle, ElementInputHandler::new(bounds, entity), cx)`, y avisa de cambios con `window.notify_text_input_changed(..)` y `notify_text_selection_changed(..)`. El texto marcado (composición) se subraya con `TextRun{ underline: Some(UnderlineStyle{..}) }`.

La PoC **adapta ese ejemplo** (Apache-2.0; procedencia anotada en la cabecera del archivo, T067) a un `TextInput` reutilizable que sirve a la celda en edición y a la barra de fórmulas, con un único búfer de edición compartido para que ambos muestren el mismo texto. Se añaden: Inicio/Fin, selección con Mayús, copiar/pegar dentro del editor (`cx.read_from_clipboard()`/`cx.write_to_clipboard(ClipboardItem::new_string(..))` `[VERIFICADO]`), Intro/Tab/Esc como acciones de la celda, y anclaje de los candidatos del IME con `bounds_for_range` (a la celda, no a la ventana). Las ediciones largas (`CommitEdit` en A2) corren en el ejecutor de fondo; las teclas mientras tanto se encolan.

Casos límite (mismos que en §6.6): composición que sobrevive al desplazamiento, Intro durante la composición, clic en otra celda durante la composición. **`[NO VERIFICADO]`** la calidad del IME de GPUI en Windows y Linux (D-07, SC-004): se prueba pronto en T008 (macOS) y en T098/T103.

### 7.6 Atajos

Acciones con `actions!(grid, [Copy, Paste, Undo, Redo, Open, Save, SaveAs, EditCell, MoveUp, …])` y enlaces con `KeyBinding::new("cmd-c", Copy, Some("Grid"))` `[VERIFICADO]` (ejemplo `input.rs`: `actions!`, `KeyBinding::new`, `.key_context(..)`, `.on_action(cx.listener(..))`, `cx.bind_keys`). El archivo `keymap.rs` define **dos tablas** seleccionadas con `cfg!(target_os = "macos")` (cmd-) y el resto (ctrl-), con los mismos atajos que §6.7. Esos enlaces solo cubren teclas con modificadores y de edición; el texto que se teclea llega por `EntityInputHandler`.

### 7.7 Diálogos y portapapeles

`cx.prompt_for_paths(PathPromptOptions{ files: true, directories: false, multiple: false, prompt: None })` y `cx.prompt_for_new_path(..)` `[VERIFICADO]` (`crates/gpui/src/app.rs`, `platform.rs`). Portapapeles: `cx.read_from_clipboard()` / `cx.write_to_clipboard(..)` `[VERIFICADO]`.

### 7.8 Instrumentación interna

Duración de `prepaint` y `paint` por cuadro; intervalo entre cuadros consecutivos (con `window.request_animation_frame()` durante el desplazamiento); `window.refresh_interval(cx)` se registra como frecuencia de refresco declarada del informe `[VERIFICADO]` que existe. Latencia de tecla: `replace_text_in_range` → fin del siguiente `paint`.

### 7.9 Lo que no está verificado de GPUI

Estabilidad de la API entre commits (pre-1.0; el README lo advierte: «There will often be breaking changes between versions» `[VERIFICADO]`); que `ContentMask` y `with_content_mask` tengan la forma supuesta; el consumo de CPU de repintar a 60/120 Hz sin invalidación; el comportamiento de GPUI en Linux con NVIDIA (existe el crate `gpui_wgpu` y el script de dependencias de Zed instala `libvulkan1` `[VERIFICADO]`, de modo que se espera Vulkan; que sea obligatorio y qué controlador necesita `[NO VERIFICADO]`); compilación en Windows (requisitos de Visual Studio y CMake `[NO VERIFICADO]`); coste de compilar la clausura de Zed en CI.

---

## 8. Paridad funcional entre interfaces

Las dos apps ofrecen lo mismo; si algo se hace en una y no en la otra, el informe no es válido.

| FR | Tauri | GPUI | Prueba común |
|---|---|---|---|
| FR-001 | `--open` + `tauri-plugin-dialog` | `--open` + `prompt_for_paths` | guion `open` + `assert_cell` |
| FR-002 | canvas 2D desde DrawList | `GridElement` desde DrawList | comparación visual con ONLYOFFICE de una región de A1 (T053, T058, T063) |
| FR-003 | caché + espaciador escalado | `on_scroll_wheel` + barras propias | invariante `cells_drawn_per_frame` (T032, T063) |
| FR-004 | `input.ts` | `keymap.rs` + ratón | guiones `us1` |
| FR-005 | textarea editor + barra de fórmulas | `TextInput` + `EditorElement` | guiones `us2` |
| FR-006 | `CommitEdit` (núcleo) | ídem | lista fija de fórmulas es/en (T024, T070) |
| FR-007 | textarea + `ime.ts` | `EntityInputHandler` | manual SC-004 (T072, T098, T103) |
| FR-008 | atajos → `Undo/Redo` | ídem | guiones `us2`, T025 |
| FR-009 | `tabs.ts` | `tabs.rs` | guiones `us3` sobre A3 |
| FR-010 | plugin de portapapeles | `ClipboardItem` | núcleo (T027) + manual entre apps |
| FR-011 | `Save`/`SaveAs` | ídem | ida y vuelta (T077) |
| FR-012 | `metrics.ts` + `dokumentz-metrics` | `metrics.rs` + `dokumentz-metrics` | T013, T056, T061 |
| FR-013 | — (herramienta `testdata-gen`) | — | T038 |
| FR-014 | — (herramienta `compat-check`; no depende de ninguna interfaz) | — | autoprueba con fixtures y corrida sobre A4 (T115, T116) |


---

## 9. Método de medición

Principios generales en D-11. Aquí se concretan. Las tablas de resultados son **por plataforma**; no se extrapola de una a otra. Los números internos y externos se guardan con `method` distinto y **no se mezclan** al comparar contra ONLYOFFICE. M-09 (compatibilidad) no mide tiempo y tiene su propio método (§9.10); sus resultados se guardan con `method: "compat"`.

### 9.1 Reglas para que sea justo con ONLYOFFICE

1. **Misma máquina, misma pantalla.** Resolución, escala y frecuencia de refresco se registran en cada JSON. Ventana de **1280×720 px lógicos** en la posición (80, 60), colocada por `bench-runner` con la API del SO para las tres apps. Tema claro, zoom 100 %.
2. **Mismos archivos y operaciones.** A1–A3 de `testdata-gen` con la misma semilla (`sha256` en cada resultado); las mismas secuencias de entrada sintética para las tres apps; idioma de fórmulas `es`.
3. **Mismas definiciones** de «ventana usable» y «primera pantalla con datos», basadas en el contenido de la pantalla (§9.3), no en señales internas.
4. **ONLYOFFICE 10.0.0** en macOS (versión en Windows/Linux: aclaración 3), configuración por defecto. Único ajuste previsto: desactivar la comprobación de actualizaciones para que no genere red ni CPU durante la medición; se declara en el informe. Se abre con la ruta como argumento `[A VERIFICAR EN T084]`.
5. **Compilaciones `release`** de las PoC con el mismo perfil; nunca `debug`.
6. **Mismo sobrecosto de captura**: la captura de pantalla está activa en todas las corridas de M-01…M-05 y M-07 para las tres apps; **M-06 y M-08 se miden sin captura**. `bench-runner calibrate` mide el «piso del método» (latencia y jitter de captura+entrada sobre una ventana de referencia mínima) y se informa sin restarlo.
7. **Orden intercalado.** Las 5 repeticiones se reparten en rondas (app A, B, ONLYOFFICE, A, B, ONLYOFFICE…) con 30 s de enfriamiento entre repeticiones, para repartir la deriva térmica y de fondo.
8. **Estado de la máquina** (comprobado por `bench-runner preflight` y registrado): conectada a corriente, sin otras aplicaciones pesadas (CPU ociosa ≥ 90 % durante 30 s), notificaciones y reposo desactivados, barras de desplazamiento del SO en «siempre» (macOS), una sola pantalla activa, VM = falso.
9. **Alcance distinto declarado, sin corregir los datos:** ONLYOFFICE incluye procesador de textos, presentaciones y cinta (M-08, M-06, M-01). El informe lo escribe en una nota fija.
10. **Se reportan también las medidas en las que la PoC pierde** (principio VIII) y las repeticiones invalidadas.

### 9.2 Qué es externo y qué es interno

| Origen | Cómo | Para qué | Se usa en SC |
|---|---|---|---|
| **Externo** (las tres apps) | `bench-runner`: lanza el proceso, coloca la ventana, inyecta entrada del SO, captura pantalla a tasa constante, analiza cuadros, muestrea memoria | Comparar con ONLYOFFICE | **Sí** (SC-001…SC-003) |
| **Interno** (solo las dos PoC) | `dokumentz-metrics` con `--metrics-out` | Diagnosticar dónde se va el tiempo: carga, `draw_block`, puente, pintado | No para comparar con ONLYOFFICE |
| **Verificación cruzada** | PresentMon (Windows), `xctrace` «Animation Hitches» (macOS), MangoHud (Linux, opcional) | Confirmar el orden de magnitud de M-03 con una fuente independiente de la captura | No decide |

### 9.3 Definición operativa de cada medida (externa)

«Estado estable» = el contenido de la región de interés no cambia (diferencia de píxeles por debajo de ε) durante ≥ 300 ms. «Cuadro final» = el primero de ese estado estable. Las regiones se calculan a partir de la posición fija de la ventana y de coordenadas por aplicación en `tools/bench-runner/apps.toml` (cuadrícula, barra de fórmulas, pista de la barra de desplazamiento), que se verifican con una captura guardada en `results/` en la primera repetición.

| Medida | Entrada del driver | t0 | t1 | Notas |
|---|---|---|---|---|
| **M-01** (oficial, variante B) | Lanzar con `empty.xlsx` como argumento | Reloj de pared justo antes de crear el proceso | Cuadro final de la **cuadrícula con cabeceras** visible; después una tecla de prueba que debe aparecer en la celda en ≤ 2 s (si no, repetición inválida; la tecla no cuenta en t1) | «Usable» = editor de hojas con un libro vacío listo. Igual para las tres apps |
| **M-01-inicio** (secundaria, variante A) | Lanzar sin archivo | ídem | PoC: cuadrícula vacía. ONLYOFFICE: pantalla de inicio estable | Se declara que no son equivalentes; sirve para ver el arranque puro |
| **M-02** | Lanzar con A1 como argumento (frío y caliente) | Lanzamiento | Cuadro final de la región de la cuadrícula **con datos** (distinto del estado vacío previo y sin marcadores de carga) | «Costo de abrir» derivado = M-02 − M-01. La apertura por diálogo (FR-001) se verifica funcionalmente, no se cronometra de forma externa |
| **M-03** | **S1:** rueda sintética (en píxeles donde el SO lo permite) a 60 eventos/s durante 10 s. **S2:** arrastre del pulgar de la barra vertical, de arriba abajo, en 10 s con movimientos a 120 Hz | Inicio del movimiento | 10 s después | Captura a tasa constante (≥ frecuencia de refresco). Se calculan los intervalos entre **cambios visuales consecutivos** de la cuadrícula: p95, p99, número de cuadros perdidos (= intervalos > 1,5 × periodo de refresco) y fps efectivos. Evaluación de SC-002 con el **peor** de S1 y S2. El trackpad real no se puede sintetizar de forma igual en todos los SO: se prueba a mano y se anota |
| **M-04** | En A1 y en A2: seleccionar una celda y teclear 100 mayúsculas anchas (`M W Q O B H` en ciclo) con intervalos pseudoaleatorios de 120–200 ms (semilla fija) | Reloj de pared de la inyección de cada tecla | Primer cuadro en que la región de edición cambia más que el parpadeo del cursor (umbral de píxeles cambiados) | Se usan letras anchas para separar el cambio del parpadeo. Latencia = t1 − t0 por tecla; p95 por repetición. Incluye el camino del SO (spec §6) |
| **M-05** | Sobre A2: `Derecha ×20` desde A1 (celda de entrada `U1`), teclear `7`, `Intro` | Inyección de `Intro` | Cuadro final de la región de las dependientes (columnas `V`–`W`, filas 2–40, **excluyendo** la selección) | No se usa «primer cambio»: la selección se mueve al pulsar Intro. Si las dependientes no cambian, la repetición es inválida |
| **M-06** | Abrir A1, esperar 5 s tras la primera pantalla | — | Muestreo cada 250 ms durante 10 s en reposo | Mediana de las muestras (M-06 oficial); el pico durante M-03 se informa aparte. **Sin captura de pantalla**. Detalle en §9.5 |
| **M-07** | En una copia de A1: teclear `1` + `Intro` en una celda y pulsar Cmd/Ctrl+S | Inyección de la tecla de guardar | El archivo termina de escribirse: tamaño y `mtime` estables durante 200 ms (sondeo a 1 ms) y es un zip válido | Después `xlsx-diff` contra «A1 + la edición»: si hay diferencias, **la repetición cuenta como fallo** (spec §6). Frío y caliente |
| **M-08** | — | — | Tamaño en disco del directorio instalado/empaquetado | Mismo comando en las tres: `du -sk` (macOS/Linux) o suma de `Length` (Windows). Se informan aparte: instalador/paquete y que Tauri **no** incluye la webview del sistema y GPUI no incluye fuentes del sistema. ONLYOFFICE: la medida local de 941 MB se repite con el mismo método |

Frío / caliente (spec §6): **frío** = tras vaciar la caché de archivos del SO y sin haber ejecutado la app ni abierto el archivo en esa sesión; **caliente** = lanzar de nuevo justo después. Aplica a M-01, M-02 y M-07; el resto se mide en caliente.

### 9.4 Herramientas por plataforma

| Función | macOS | Windows | Linux |
|---|---|---|---|
| Captura principal | `ffmpeg -f avfoundation -framerate N -capture_cursor 0` `[VERIFICADO]` que las opciones existen (ffmpeg.org/ffmpeg-devices.html); la guía **no** documenta la tasa máxima, `[A VERIFICAR EN T042]` si alcanza 2× de la frecuencia de refresco. Plan B: ayudante en Swift con ScreenCaptureKit y `SCStreamConfiguration.minimumFrameInterval` (introducido en macOS 12.3 `[NO VERIFICADO]` con la fuente de Apple, que no cargó) | `ffmpeg -f lavfi -i ddagrab=framerate=N:dup_frames=true:video_size=WxH:offset_x=X:offset_y=Y` seguido de `hwdownload,format=bgra`. `ddagrab` usa Desktop Duplication y entrega fotogramas D3D11 `[VERIFICADO]` (ffmpeg-filters). Alternativa: `gfxcapture` (Windows.Graphics.Capture) `[VERIFICADO]` | `ffmpeg -f x11grab -framerate N -video_size WxH -i :0.0+X,Y` en **sesión X11** `[VERIFICADO]`. `kmsgrab` exige privilegios y no sincroniza con el cambio de buffer `[VERIFICADO]` |
| Marcas de tiempo | `-use_wallclock_as_timestamps 1` para que el reloj de pared de `ffmpeg` y el del driver coincidan; la cuantización la fija la tasa de captura | ídem | ídem |
| Entrada sintética | `CGEvent` (`CGEventPost`; rueda con `CGEventCreateScrollWheelEvent2` en píxeles) desde Rust. Requiere los permisos de Grabación de pantalla y Accesibilidad para el terminal, que concede la persona (no se automatiza) `[NO VERIFICADO]` | `SendInput`. El proceso del driver no debe estar elevado respecto de la app (UIPI) `[NO VERIFICADO]` | XTest (`xdotool` o `x11rb`) en sesión X11. En Wayland nativo no se puede inyectar `[NO VERIFICADO]` |
| Colocar y dimensionar la ventana | API de accesibilidad / AppleScript `System Events` | `SetWindowPos` | `xdotool windowsize/windowmove` |
| Frecuencia de refresco | `system_profiler SPDisplaysDataType` mostró la resolución pero **no** la frecuencia en este Mac `[VERIFICADO]`; se pide a `CGDisplayCopyDisplayMode` desde `bench-runner env` `[NO VERIFICADO]`; respaldo: `window.refresh_interval` de GPUI | `Win32_VideoController.CurrentRefreshRate` | `xrandr` |
| Verificación cruzada de cuadros | `xcrun xctrace record --template "Animation Hitches"` (la plantilla existe en este Mac junto con «Metal System Trace», «App Launch» y «Activity Monitor» `[VERIFICADO]`) | **PresentMon** 2.x: `--process_name`/`--process_id`, `--output_file`, `--timed N`, `--terminate_after_timed`, `--qpc_time`; columnas `MsBetweenPresents`, `MsBetweenDisplayChange`, `DisplayLatency`, `DisplayedTime` `[VERIFICADO]` (README de la consola). Requiere pertenecer al grupo «Performance Log Users» `[VERIFICADO]`. Los motores Chromium (WebView2, CEF) presentan desde procesos auxiliares: se filtra por el árbol de procesos de la app `[A VERIFICAR EN T095]` | MangoHud (`output_folder`, `log_duration`, `autostart_log`) `[VERIFICADO]` por documentación de terceros; solo si el motor dibuja con Vulkan/OpenGL enganchable; opcional |
| Vaciar la caché de archivos (frío) | `sudo purge` | reinicio, o RAMMap | `sync; echo 3 > /proc/sys/vm/drop_caches` como administrador | 

(`purge`, RAMMap y `drop_caches` son de conocimiento general, `[NO VERIFICADO]` en esta sesión; el RUNBOOK de T083 los marca como pasos con privilegios que ejecuta la persona.)

### 9.5 Memoria total de todos los procesos (M-06)

Métrica por plataforma, igual para las tres apps de esa plataforma (D-11):

| Plataforma | Métrica por proceso | Qué procesos se suman |
|---|---|---|
| macOS | `phys_footprint` de `footprint -a -j <archivo>` `[VERIFICADO]` que las opciones `-a` y `-j` existen | (1) El árbol del proceso lanzado por `ppid`. (2) Los servicios `com.apple.WebKit.WebContent`, `.GPU` y `.Networking` **nuevos** respecto de la línea base tomada antes de lanzar (no cuelgan del proceso de la app; en este Mac ya hay otros vivos de otras apps `[VERIFICADO]` con `pgrep`, de ahí la diferencia de PIDs y la regla de cerrar Safari y otras apps WebKit). ONLYOFFICE: `ONLYOFFICE` + `editors_helper`, `editors_helper (GPU)`, `editors_helper (Renderer)` `[VERIFICADO]` en el paquete local |
| Windows | Conjunto de trabajo privado | Árbol por proceso padre, incluidos los `msedgewebview2.exe` de la app y los auxiliares de CEF `[NO VERIFICADO]` |
| Linux | `Pss` de `/proc/<pid>/smaps_rollup` | Árbol por proceso padre, incluidos `WebKitWebProcess` y `WebKitNetworkProcess` `[NO VERIFICADO]` |

La atribución de procesos WebKit en macOS se valida en T089 con una corrida de comprobación: lanzar PoC-Tauri, listar `footprint -a` antes y después, y confirmar que los servicios nuevos desaparecen al cerrar la app. La memoria compartida de marcos del sistema se cuenta de forma distinta en cada SO; por eso no se comparan MB entre plataformas.

### 9.6 Procedimiento de una corrida

1. `bench-runner env` y `preflight` → fallan si algo no cumple §9.1-8.
2. Calibración: `bench-runner calibrate` (una vez por sesión de medición).
3. Para cada medida y archivo: rondas intercaladas ×5, 30 s de enfriamiento; matar todos los procesos de la app al terminar cada repetición.
4. Cada repetición escribe su entrada en el JSON; una repetición inválida (ventana ausente tras 60 s, tecla de prueba que no aparece, captura con huecos, fallo de integridad en M-07) se registra con `ok:false` y se repite hasta 2 veces más; **ninguna se descarta en silencio**.
5. Capturas de verificación (primera repetición de cada app) y los vídeos de M-03 quedan en `results/…/evidencia/` (vídeos en `.gitignore`, solo hashes en el JSON).
6. `bench-report` agrega y escribe el informe.

### 9.7 Formato JSON de resultados (`results/<plataforma>/<run-id>/<medida>-<app>.json`)

Los tipos viven en `dokumentz-metrics::results` y su JSON Schema en `tools/bench-report/schema/results.v1.schema.json` (T039). El mismo esquema lo escriben `bench-runner` (externo) y las PoC con `--metrics-out` (interno).

```json
{
  "schema": "dokumentz-bench/1",
  "run_id": "2026-11-03T10-12-00Z-macos-m1",
  "criteria_sha": "<hash del commit etiquetado criterios-v1>",
  "method": "external",                       // "external" | "internal" | "compat" (M-09, §9.10)
  "env": {
    "os": "macOS 26.6.2", "arch": "arm64", "cpu": "Apple M1", "ram_gb": 16, "gpu": "Apple M1",
    "display": { "width_px": 2560, "height_px": 1600, "scale": 2.0, "refresh_hz": 60.0 },
    "web_engine": "WKWebView (WebKit del sistema)", "vm": false, "power": "ac", "session": "aqua"
  },
  "app": { "id": "dokumentz-app-tauri", "version": "0.1.0", "commit": "<git>", "build": "release",
           "lang": "es", "exe_sha256": "<…>" },
  "file": { "id": "A1", "sha256": "<…>", "rows": 100000, "cols": 20 },
  "measure": "M-03", "scenario": "S1-wheel-10s", "cold": false,
  "capture": { "tool": "ffmpeg", "fps": 60, "quantum_ms": 16.67, "floor_ms": null },
  "reps": [
    { "n": 1, "ok": true, "samples": 598, "p50": 16.7, "p95": 16.7, "p99": 33.3,
      "dropped": 4, "effective_fps": 59.8, "notes": "" }
  ],
  "summary": { "median_of_p95": 16.7, "worst_p95": 33.3, "median": 16.7, "p95": 33.3, "min": 16.7, "max": 33.3 },
  "invalidated": []
}
```

Las medidas de tiempo único (M-01, M-02, M-05, M-07) llevan `value_ms` por repetición y `summary` con mediana, p95, mínimo y máximo; M-06, `value_mb` (mediana en reposo) y `peak_mb`; M-08, `size_mb` y `installer_mb`. Agregación (spec §6): para M-03 y M-04, p95 por repetición y mediana entre repeticiones más el peor; para el resto, mediana y p95 de las cinco con mínimo y máximo.

### 9.8 Informe comparativo

`bench-report --results results/<plataforma>/<run-id> --criteria tools/bench-report/criteria.toml` escribe `informe.json` y `informe.md`:

1. Entorno (hardware, SO, versión del motor web, frecuencia de refresco, si es VM) y versiones de las tres apps.
2. Tabla por medida × app: mediana, p95, mínimo/máximo, repeticiones válidas; razón frente a ONLYOFFICE; frío y caliente por separado.
3. Estado de **SC-001…SC-007**: `cumple` / `no cumple` / `sin datos`, con el dato que lo sustenta (lo no medido nunca cuenta como cumplido). SC-007 sale de `M-09-compat.json` (§9.10) y se muestra también **por fuente**.
4. Aplicación de la **regla de decisión de interfaz** de `spec.md` §9 en el orden 1→4, con la rama tomada y los datos. Los casos «no cubiertos» (aclaración 9) salen como `NO CUBIERTO: decide el dueño`. SC-007 **no interviene** en esta regla.
4b. Aplicación de la **regla del motor** de `spec.md` §9 (ramas M1, M2, M3 o `sin datos`) en una sección aparte, con la lista de funciones faltantes o distintas (M-09(d)) y la rama tomada. Las dos reglas se evalúan por separado y se muestran por separado.
5. «Dónde pierde la PoC» (medidas en que ONLYOFFICE gana) y la nota fija de alcance.
6. Para Linux con NVIDIA: configuración y variables de entorno aplicadas, de modo que no se confundan con el comportamiento por defecto.
7. Comparación interna (diagnóstico) en una sección aparte, con la leyenda «no comparable con ONLYOFFICE».

`criteria.toml` contiene SC-001…SC-005 (umbrales, factor 2×, 50 %, 16,7 ms, 30 ms), SC-007 (95 % de archivos sin error y 0 cierres; 98 % de fórmulas coincidentes; 100 % de valores sin fórmula idénticos; umbral 90 % de la regla del motor; tolerancia numérica relativa 1e-9), la lectura de aclaraciones 9 y 10 y la tolerancia de «≥2× en M-03/M-04». SC-006 lo evalúa la CI y no tiene umbral numérico. Se congela con la etiqueta `criterios-v1` y su hash viaja en cada JSON (`criteria_sha`); cambiarlo después de la primera corrida oficial exige un `criterios-v2` y declararlo en el informe.

### 9.9 Límites conocidos del método

- La captura cuantiza el tiempo: p95 de 16,7 ms significa «a lo sumo un periodo de refresco», no una latencia de GPU. En un Mac de 60 Hz no se puede distinguir 120 Hz (SC-002 ideal queda `sin datos` hasta tener ProMotion).
- La latencia externa de teclado es una **cota superior** (incluye el camino del SO y el retraso de captura); la comparación entre apps es válida, el valor absoluto no. Se valida opcionalmente con una cámara de 240 fps sobre una plataforma.
- La rueda sintética no equivale a un trackpad real (inercia). S2 (arrastre de barra) reduce la diferencia de semántica entre apps.
- La atribución de procesos para M-06 en macOS es por diferencia de PIDs; se valida en T089.
- Sin hardware Windows/Linux real, esas columnas quedan `sin datos` y la regla de decisión no se puede aplicar completa (R-06).
- **M-09 (límites del método de compatibilidad, §9.10).** (1) `Application` de `docProps/app.xml` no garantiza que Excel calculara los valores guardados: las bibliotecas que generan `.xlsx` copian «Microsoft Excel» y guardan valores provisionales (hallazgo 4). (2) Un libro de Excel en cálculo manual puede traer valores obsoletos. (3) Las dos cosas se registran en el JSON (`calcPr`). Resuelto por defecto (2026-10-22): `fullCalcOnLoad="1"` o la ausencia total de valores guardados sacan el archivo de (b) como `generador_sospechoso` (§9.10, paso 1); `calcMode="manual"` se informa como «valores guardados posiblemente obsoletos», pero el archivo sigue en (b). (4) El filtro de volátiles no sigue nombres definidos, `INDIRECT` ni `OFFSET`. (5) Excel redondea algunas restas casi nulas a 0 (p. ej. `0,1+0,2−0,3`) y con tolerancia relativa pura saldrían como discrepancias; por eso §9.10 añade un piso absoluto de 1e-12. (6) Un resultado de M-09 describe al motor y a los archivos de A4; no se extrapola a «todos los archivos del mundo».

### 9.10 Compatibilidad con archivos reales (M-09, SC-007)

No mide tiempo: no usa captura, entrada sintética ni repeticiones. Lo ejecuta `tools/compat-check` (FR-014) en local sobre A4, y es **determinista**: una corrida por archivo (se repite solo para comprobar que el resultado no cambia). Razones del método y fuentes: D-17.

**Entradas.** `testdata/a4/manifest.toml` (T114) lista por archivo `id`, `fuente` (`usuario` | `apache-poi` | `libreoffice`), licencia y SHA-256. Los archivos están en `testdata/a4/` y no salen del equipo.

**Procedimiento por archivo.** El supervisor lanza un proceso hijo por archivo con un tiempo máximo (propuesta: 120 s); registra salida normal, error de apertura, tiempo agotado o cierre inesperado (terminación por señal, aborto o pánico no controlado) y sigue con el siguiente. Pasos del hijo:

1. **Productor.** Lee `docProps/app.xml` con el crate `zip` y extrae `<Application>`: `excel` si empieza por «Microsoft Excel»; `libreoffice` si contiene «LibreOffice»; `poi` si contiene «Apache POI»; `otro` si trae otro valor; `desconocido` si no hay `app.xml` o `Application`. Guarda el texto literal. **Segunda señal (resuelta por defecto el 2026-10-22):** un archivo con `Application` «Microsoft Excel» pasa a `generador_sospechoso` si su `calcPr` de `xl/workbook.xml` trae `fullCalcOnLoad="1"` o si ninguna de sus celdas con fórmula trae valor guardado (así lo dejan `rust_xlsxwriter` y XlsxWriter). Solo `excel` entra en (b); `generador_sospechoso` y el resto se informan aparte.
2. **Referencia independiente.** `calamine` 0.37.0 lee hoja por hoja los valores guardados (`worksheet_range`) y las fórmulas (`worksheet_formula`: texto sin `=`, nombres de función en inglés como en el archivo, a veces con prefijo `_xlfn.`) `[VERIFICADO]` (D-17). Una celda es «con fórmula» si calamine le da fórmula no vacía; las macros no se leen (no se llama a `vba_project`).
3. **Abrir con el motor.** `Core::open_xlsx` (misma ruta que la aplicación) con idioma `en` y zona `UTC`, de modo que los errores de función desconocida sean `#NAME?` y no `#¿NOMBRE?`. Un error de apertura queda como `opened=false` con el motivo.
4. **Recalcular.** Llamada explícita a la evaluación completa y lectura del valor resultante (`inspect.rs`, §5.9). `[A VERIFICAR EN T115]` que el valor leído viene del recálculo.
5. **Comparar celdas con fórmula** (solo `excel`; sin volátiles; con valor guardado):

   | Tipo del valor guardado | Coincide si |
   |---|---|
   | Número (las fechas, como número de serie) | `abs(a − b) ≤ 1e-9 · max(abs(a), abs(b))`; 0 contra 0 coincide |
   | Texto | Igualdad exacta de la cadena |
   | Booleano | Igualdad exacta |
   | Error | Mismo tipo de error (`#DIV/0!`, `#N/A`, `#NAME?`, `#NULL!`, `#NUM!`, `#REF!`, `#VALUE!`…) |
   | Tipos distintos (p. ej. número contra texto) | Nunca coincide: discrepancia de tipo |

   **Piso absoluto (resuelto por defecto el 2026-10-22):** dos números también coinciden si `|a − b| ≤ 1e-12`, para no contar como incompatibles los residuos de coma flotante cerca de cero (Excel muestra 0 en esos casos). Las celdas que coinciden **solo** por el piso se cuentan aparte (`near_zero`) para poder revisarlas.
6. **Volátiles.** Se excluyen de (b) y se cuentan aparte (`volatile_excluded`) las celdas cuya fórmula contiene `TODAY`, `NOW`, `RAND`, `RANDBETWEEN`, `RANDARRAY`, `CELL` o `INFO` (nombres sin `_xlfn.`), porque su valor depende del reloj, del azar o del entorno y no puede coincidir con el guardado, y las celdas que dependen de ellas (cierre transitivo con un extractor de referencias de celda y de rango, también entre hojas). `OFFSET` e `INDIRECT` son volátiles para el recálculo pero deterministas: **sí** se comparan; sus dependientes por nombre definido o por `INDIRECT`/`OFFSET` no se siguen y, si discrepan, llevan la marca «posible dependiente de volátil».
7. **Sin valor guardado.** Si el archivo trae la fórmula sin resultado, la celda cuenta en `no_stored_value`, no entra en el denominador de (b) y no es un fallo. `[A VERIFICAR EN T115]` cómo distingue calamine «sin `<v>`» de un resultado de texto vacío.
8. **Celdas sin fórmula** (todos los productores): el valor leído del motor tras abrir debe ser **idéntico** al de calamine (números por igualdad de `f64`; texto, booleano, error literal y fecha como número de serie por igualdad exacta). Las celdas vacías no cuentan. M-09(c) = idénticas / celdas con valor sin fórmula, en archivos abiertos.
9. **Funciones faltantes o distintas (d).** Por cada celda que da `#NAME?` o discrepa, extrae los nombres de función de la fórmula guardada y acumula por función: `celdas_afectadas`, `archivos_afectados`, `tipo` (`no_soportada` si el motor devuelve `#NAME?` y su analizador no reconoce el nombre; `resultado_distinto` si calcula pero difiere) y hasta 3 ejemplos (identificador de archivo, hoja, celda). Orden: `celdas_afectadas` descendente, luego `archivos_afectados`, luego nombre. `[A VERIFICAR EN T115]` cómo preguntar al motor si un nombre de función existe.
10. **Macros: nunca.** El binario no ejecuta VBA ni XLM, no llama a `vba_project` de calamine y no abre ningún archivo con otra aplicación. Los `.xlsm` se abren como cualquier libro (prueba con un `.xlsm` ficticio, T115).

**Métricas y veredicto.**
- (a) = archivos con `opened=true` y sin cierre / archivos totales; `crashes` = número de cierres inesperados.
- (b) = celdas coincidentes / celdas comparadas, sumadas sobre archivos `excel`.
- (c) = celdas sin fórmula idénticas / celdas sin fórmula con valor, en archivos abiertos.
- Se calculan en total, **por fuente** (usuario / Apache POI / LibreOffice) y **por productor**. SC-007 se evalúa sobre el **total**; el desglose por fuente no cambia el veredicto, pero los archivos del usuario pesan más al interpretarlo (los corpus de POI y LibreOffice incluyen archivos raros o de reportes de errores).
- `cumple` solo si (a) ≥ 95 % con 0 cierres, (b) ≥ 98 % y (c) = 100 %; `sin datos` si A4 no está preparado o no hay ningún archivo `excel` con fórmulas con valor guardado.
- Regla del motor (`spec.md` §9), implementada en `bench-report` (T117): **M1** cumple → motor validado; **M2** no cumple y (b) ≥ 90 % → plan de motor con la lista (d) para contribuir a IronCalc y volver a medir antes de la fase 1; **M3** (b) < 90 % → además, evaluar alternativas de motor de fórmulas; sin datos → motor no validado.

**Salida** (`results/<plataforma>/<run-id>/M-09-compat.json` y `compat.md`; mismo esquema `dokumentz-bench/1`, con `method: "compat"` y `measure: "M-09"`; el JSON Schema se amplía en T117):

```json
{
  "schema": "dokumentz-bench/1", "run_id": "…", "criteria_sha": "…", "method": "compat", "measure": "M-09",
  "env": { "os": "macOS 26.6.2", "arch": "arm64", "vm": false },
  "engine": { "name": "ironcalc", "version": "0.9.1", "dokumentz_sheet_commit": "<git>", "lang": "en", "tz": "UTC" },
  "reader": { "name": "calamine", "version": "0.37.0" },
  "tolerance": { "number_rel": 1e-9 },
  "corpus": { "manifest_sha256": "<…>", "files_total": 0 },
  "files": [
    { "id": "<12 hex de SHA-256>", "source": "usuario|apache-poi|libreoffice",
      "producer": "excel|libreoffice|poi|otro|desconocido", "application": "Microsoft Excel", "calc_pr": { "fullCalcOnLoad": false, "calcMode": "auto" },
      "opened": true, "error": null, "crash": false, "macros_executed": false,
      "formula_cells": 0, "compared": 0, "matched": 0, "mismatched": 0, "volatile_excluded": 0, "no_stored_value": 0,
      "value_cells": 0, "value_identical": 0 }
  ],
  "functions": [ { "name": "XLOOKUP", "type": "no_soportada", "cells": 0, "files": 0, "examples": [] } ],
  "summary": { "total": { "a": 0.0, "crashes": 0, "b": 0.0, "c": 0.0 }, "by_source": {}, "by_producer": {} }
}
```

Privacidad (R-14: repositorio público): en los archivos de la fuente `usuario` el JSON versionado no incluye valores de celda, nombres de hoja ni rutas; los ejemplos de función llevan solo identificador de archivo y celda. El detalle celda a celda va a `results/<plataforma>/<run-id>/privado/` (ignorado por git).

**Autoprueba.** `cargo test -p compat-check` con fixtures pequeños generados con `rust_xlsxwriter` + `set_formula_result` (el valor guardado se fija a propósito, p. ej. `=A1+B1` guardado como 7 cuando el motor calcula 5): coincidente, discrepante, error igual y distinto, volátil y su dependiente, sin valor guardado (se reescribe el ZIP para quitar `<v>`), productor Excel (`rust_xlsxwriter` escribe «Microsoft Excel» `[VERIFICADO]`) y no Excel (se reescribe `app.xml`), archivo corrupto, supervisor ante un hijo que aborta, `.xlsm` ficticio, y dos corridas con el mismo JSON.

---

## 10. Integración continua (GitHub Actions)

No mide rendimiento. Ejecutores `[VERIFICADO]`: `ubuntu-24.04`, `windows-2025`, `macos-26`. Las acciones se fijan por SHA (versiones en T011/T081). Permisos `contents: read`; sin secretos; `concurrency` cancela ejecuciones obsoletas. Requiere un repositorio remoto (aclaración nueva de §2.8).

```yaml
# .github/workflows/ci.yml  (esquema)
jobs:
  core:        # SC-006 (núcleo y herramientas)
    strategy: { matrix: { os: [ubuntu-24.04, windows-2025, macos-26] } }
    steps: [checkout, rust 1.99.0 (rust-toolchain.toml), cache,
            cargo fmt --check (solo Linux), cargo clippy --workspace --exclude dokumentz-app-tauri --exclude dokumentz-app-gpui -D warnings,
            cargo test -p dokumentz-sheet -p dokumentz-metrics -p dokumentz-script -p testdata-gen -p xlsx-diff -p compat-check -p bench-runner -p bench-report,
            cargo test -p dokumentz-sheet --test no_ui_deps (solo Linux),  cargo deny check licenses bans sources (solo Linux)]
  # compat-check (autoprueba con fixtures) corre en el trabajo `core`; la corrida sobre A4 es local y nunca en CI (A4 no sale del equipo)
  roundtrip:   # SC-005 reducido
    strategy: { matrix: { os: [...] } }
    steps: [testdata-gen --scale ci --seed 42, cargo test -p dokumentz-sheet --test roundtrip, xlsx-diff sobre los A1–A3 reducidos: original vs. guardado]
  tauri:
    strategy: { matrix: { os: [...] } }
    steps: [apt (solo Linux): libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev
                              libayatana-appindicator3-dev librsvg2-dev xvfb,          # lista oficial de Tauri [VERIFICADO]
            setup-node 22, npm ci, npm test (Vitest + mockIPC),
            cargo test -p dokumentz-app-tauri, cargo build -p dokumentz-app-tauri --release,
            E2E: Linux (xvfb-run + tauri-driver + WebKitWebDriver) y Windows (tauri-driver + msedgedriver); macOS: modo guion,
            dokumentz-script-check sobre results de los guiones tests/functional/**]
  gpui:
    strategy: { matrix: { os: [...] } }
    steps: [apt (solo Linux): lista de script/linux de Zed [VERIFICADO] recortada en T081
                              (gcc g++ libfontconfig-dev libwayland-dev libx11-xcb-dev libxkbcommon-x11-dev libvulkan1
                               libzstd-dev libssl-dev cmake clang lld make ... + xvfb mesa-vulkan-drivers),
            cargo test -p dokumentz-app-gpui (característica test-support de gpui, sin GPU),
            cargo build -p dokumentz-app-gpui --release,
            humo de ventana con --script (Linux: xvfb + Vulkan por software; continue-on-error hasta demostrar estabilidad)]
  judge-libreoffice:   # opcional, solo Linux: soffice --headless --convert-to csv sobre los archivos guardados  [NO VERIFICADO en esta sesión]
```

Notas:

- Dependencias de sistema para **WebKitGTK** en Linux: `libwebkit2gtk-4.1-dev` y el resto de la lista de arriba `[VERIFICADO]` (v2.tauri.app/start/prerequisites/). En Windows se necesita WebView2 (preinstalado en Windows recientes) y las herramientas de C++ de MSVC; en macOS, Xcode o sus herramientas de línea de comandos `[VERIFICADO]`.
- Para GPUI, macOS: Xcode; la característica `runtime_shaders` de `gpui_platform` evita requerir el compilador de Metal `[VERIFICADO]` que existe; Windows: no necesita características de `gpui_platform` `[VERIFICADO]` (README), pero los requisitos de compilación (Visual Studio, CMake) son `[NO VERIFICADO]` y T081 los fija.
- Las pruebas de ventana de GPUI y de Tauri en CI son de **humo funcional**: los ejecutores no documentan GPU `[VERIFICADO]` y las VM no sirven para rendimiento (D-13).
- El tiempo de compilación de la clausura de Zed es alto; la caché de `target/` por SO y el uso de `--release` solo donde hace falta lo mitigan (R-03).

---

## 11. Riesgos y mitigaciones

| ID | Riesgo | Prob. | Impacto | Mitigación | Señal / tarea |
|---|---|---|---|---|---|
| R-01 | **Costo del puente (IPC) en Tauri.** Cada `invoke` es un `fetch` al protocolo `ipc` `[VERIFICADO]`; si el desplazamiento dependiera de él, M-03 fallaría | Media | Alto | El desplazamiento no usa el puente mientras el área visible esté dentro del bloque en caché (§6.3); respuesta binaria sin JSON; precarga predictiva; `Channel` para empujar bloques; medir `ipc_echo` pronto (**CG-2**). Plan de salida: regla de decisión → GPUI; emergencia: Tauri 3 + CEF (D-10) | T009, T054, T063 |
| R-02 | **WebKitGTK + NVIDIA** (ventana en blanco, parpadeo, caída a modo lento de canvas/WebGL sin aviso) | Media | Alto | Probar en Linux con NVIDIA real (T104); mitigaciones por variable de entorno (p. ej. `WEBKIT_DISABLE_DMABUF_RENDERER=1`) solo como configuración **declarada** en el informe, no por defecto (D-08). Si Tauri falla en Linux/NVIDIA y GPUI cumple → GPUI | T104 |
| R-03 | **Inestabilidad de la API de GPUI** (pre-1.0, «breaking changes» frecuentes `[VERIFICADO]`), clausura de dependencias enorme, parches de Zed no heredados | Alta | Medio | `rev` fijado; no se actualiza en la PoC salvo tarea dedicada; `[patch.crates-io]` replicados y probados en las tres plataformas (T007); compilar GPUI en CI desde el primer día; el candado `Cargo.lock` se versiona | T007, T081 |
| R-04 | **IronCalc pre-1.0 y recálculo no incremental.** `set_user_input` → `evaluate()` recorre todas las fórmulas del libro y, en hojas con fórmulas, todas las celdas de la hoja `[VERIFICADO]` en `main`. En A2 (2 M celdas, ≥ 10 000 fórmulas) cada edición paga ese costo: afecta M-04 y M-05 | Alta (conocido) | Medio-Alto | Medir aislado (T031). Opciones, por orden: (a) `pause_evaluation()` + un solo `evaluate()` por edición; (b) evaluar en un hilo con valores previos y aviso; (c) contribuir recálculo incremental a IronCalc (Apache-2.0) o mantener un parche. M-05 no tiene umbral SC; si es malo, se informa como hallazgo y se planea (c) para la fase 1. **CP-1 (2026-10-23): se adopta (a) y se aplaza (c)**: T031 da ≈ 31,5 ms por `CommitEdit` en A2, todo en el recorrido de las celdas (lineal en celdas, no en fórmulas); (b) solo lo escondería | T006, T031, T071 |
| R-05 | **Memoria/tiempo de carga de IronCalc con 2 M celdas** (afecta SC-001: M-02 ≥2× y M-06 ≤50 %) | Media | Alto | **CG-1 (T005):** medir carga, RSS y guardado de un `.xlsx` de 100 000×20 antes de construir nada. Si abrir tarda > 10 s o el RSS pasa de 2 GB (umbrales de aviso propuestos), el agente principal avisa al dueño del proyecto antes de seguir | T005 |
| R-06 | **Sin hardware Windows/Linux** (el dueño solo tiene Mac) | Alta | Alto | Tareas `[HW-…]` bloqueadas hasta la aclaración 2; CI y VMs solo funcionales y rotuladas «solo funcional»; opciones a plantear: equipo prestado, alquiler de máquinas con GPU. Decisión del dueño (2026-10-22): solo Mac por ahora, así que la regla se aplica con datos de macOS y la decisión es provisional para Windows y Linux (`spec.md` §9); se deben medir antes de la fase 1 en esos sistemas | T093–T105 |
| R-07 | **IME y entrada de texto** (Tauri: orden de eventos de composición distinto por motor; GPUI: IME propio en Windows/Linux sin verificar) | Alta | Alto | Máquina de estados IME con pruebas (T066, T069); prueba manual guiada temprana en macOS (T008, T072) y luego en Windows/Linux (T098, T103); SC-004 por plataforma | T008, T066, T069, T072 |
| R-08 | **Validez de la medición externa** (cuantización, jitter, rueda ≠ trackpad, permisos TCC, regiones de interés por app) | Media | Alto | Captura a tasa constante; calibración del piso; dos escenarios de desplazamiento; verificación cruzada (PresentMon, `xctrace`); capturas de verificación guardadas; derivar solo comparaciones, no valores absolutos | T045, T084, T095 |
| R-09 | **Atribución de memoria WebKit en macOS** (los servicios no cuelgan de la app) | Media | Medio | Diferencia de PIDs con la máquina limpia; validación explícita | T089 |
| R-10 | **Conflicto de dependencias** al compartir `Cargo.lock` entre Tauri y GPUI (versiones de `windows`, parches de `async-task`/`calloop`) | Media | Medio | Detectarlo en T007/T009; plan B: `apps/gpui` como espacio de trabajo independiente con dependencias por ruta | T007, T009 |
| R-11 | **Texto en la interfaz (EX-01)**: métricas de fuente distintas entre Tauri y GPUI producen recortes y `####` distintos | Media | Bajo | Misma familia por SO; comparación visual con ONLYOFFICE; la excepción caduca al cerrar la fase 0 | T053, T058 |
| R-12 | **Licencias**: clausura de terceros de GPUI sin auditar | Media | Medio | `cargo deny` en CI y auditoría completa; los crates internos de Zed ya son Apache-2.0 `[VERIFICADO]`; licencia del proyecto resuelta: MIT OR Apache-2.0, sin dependencias GPL/LGPL/AGPL | T003, T108 |
| R-13 | **Candado del núcleo**: desplazar mientras se evalúa o se abre bloquea la interfaz | Media | Medio | Operaciones largas en hilo de trabajo; bloque en caché sirve el desplazamiento sin tocar el candado; `busy` visible | T029, T050, T057 |
| R-14 | **Repositorio público desde el inicio**: todo lo que se sube es visible | Media | Bajo | Resuelto el alojamiento (https://github.com/jahziel1/dokumentz). `.gitignore` excluye `testdata/` y las grabaciones; revisar antes de cada `push` que no se suben archivos A4 ni datos personales | T001, T011, T081 |
| R-15 | **Accesibilidad completa** fuera de la PoC | Cierta | Medio (para fase 1) | Se documenta como riesgo (spec §10). GPUI declara `accesskit` entre sus dependencias `[VERIFICADO]` en su `Cargo.toml`, y un canvas 2D no expone sus celdas a los lectores de pantalla por sí solo `[NO VERIFICADO]`; se evalúa en la fase 1 | T112 |
| R-16 | **Archivos mucho mayores que A1** y filas > 1 M visibles (aclaración 7) | Baja | Medio | Apertura en hilo con indicador; extensión escalada en Tauri; sin umbral definido | T054, T112 |
| R-17 | **Editar la verificación del método tras ver resultados** (violaría VIII) | Baja | Alto | `criteria.toml` congelado con etiqueta y `criteria_sha` en cada JSON | T049 |
| R-18 | **IronCalc no cubre funciones que sí usan los archivos reales**, o las calcula distinto de Excel (su tabla de idiomas lista 497 nombres, pero no se sabe cuántos calculan igual que Excel). Daría SC-007 en rojo y archivos que se abren con valores distintos o `#NAME?` | Media | Alto | Medirlo pronto con `compat-check` (SC-007) sobre A4, no al final; la lista M-09(d) ordenada por frecuencia es la lista de trabajo. Si SC-007 falla: regla del motor (`spec.md` §9): **M2** → plan de motor y **contribuir a IronCalc** (su licencia MIT OR Apache-2.0 lo permite): abrir issues con ejemplos mínimos sintéticos y enviar las funciones faltantes (T118), y volver a medir; **M3** ((b) < 90 %) → además, evaluar alternativas de motor de fórmulas antes de la fase 1. Mientras tanto no se afirma compatibilidad. Riesgo conexo: archivos de bibliotecas que dicen ser de Excel falsean (b) (§9.9) | T115–T118 |

---

## 12. Fases de implementación (alineadas con `tasks.md`)

| Fase | Contenido | Tareas | Historias | Compuerta de salida |
|---|---|---|---|---|
| 0. Preparación | Repositorio, toolchain, licencias, spikes de IronCalc, GPUI y Tauri, CI mínima | T001–T011 | — | **CP-0** + **CG-1** (carga de IronCalc) + **CG-2** (IPC) |
| 1. Núcleo (fundacional, bloquea lo demás) | `dokumentz-metrics`, `dokumentz-script`, `dokumentz-sheet` completo con pruebas sin ventana | T012–T032 | Todas | **CP-1**: `cargo test -p dokumentz-sheet` verde, `no_ui_deps` verde, dorado `drawlist_v1.bin` congelado |
| 2. Generador y medición | `testdata-gen`, `xlsx-diff`, `bench-runner`, `bench-report`, criterios (incluidos los umbrales de SC-007) | T033–T049 | US4 | **CP-2**: A1–A3 reproducibles; `bench-report` prueba todas las ramas de la regla; criterios listos para congelar |
| 3. US1 | Abrir y recorrer: Tauri y GPUI | T050–T063 | US1 | **CP-3**: guiones `us1` pasan en ambas; M-01/M-02/M-03 internas en el Mac |
| 4. US2 | Editar, fórmulas es/en, IME, deshacer | T064–T072 | US2 | **CP-4**: guiones `us2` pasan; M-04/M-05 internas; guion manual de IME hecho en macOS |
| 5. US3 y US5 | Hojas, copiar/pegar, guardar; compatibilidad del motor con archivos reales (`compat-check`, A4) | T073–T080, T114, T115–T118 | US3, US5 | **CP-5**: ida y vuelta A1–A3 sin diferencias; jueces; `compat-check` corrido (autoprueba con fixtures y, si A4 está preparado, sobre A4); SC-007 y regla del motor evaluadas o `sin datos` |
| 6. US4 | CI completa, medición externa (macOS ahora; Windows y Linux cuando haya hardware), informe (SC-001…SC-007; las dos reglas por separado) | T081–T107 | US4 | **CP-6** (macOS), **CP-7** (consolidado) |
| 7. Cierre | Auditorías (licencias, sin red), decisión, procedencia, actualización de documentos | T108–T113 | US4 | **CP-8**: SC-006 en verde en las tres plataformas; decisión registrada; `/clear` |

Etapas de agente (tamaño acotado, archivos que lee y toca) y puntos de control están en `tasks.md`. Las tareas de SC-007 forman la etapa **E33** (T115–T118), entre E25 y CP-5: T115 (`compat-check`) puede empezar en cuanto exista el núcleo (CP-1) y `calamine` esté fijado (T036); T116 (corrida sobre A4) necesita A4 preparado (T114, con el visto bueno del dueño) y los criterios congelados (T049) para ser oficial; sin eso, sus resultados son de desarrollo (`results/dev/`) y SC-007 sigue `sin datos`.

---

## Anexo A. Verificaciones hechas para este plan (2026-10-22)

| Dato | Resultado | Fuente |
|---|---|---|
| Función de carga `.xlsx` | `ironcalc::import::load_from_xlsx<'a>(file_name: &str, locale: &str, tz: &str, language: &'a str) -> Result<Model<'a>, XlsxError>`. También existen `load_from_icalc` y `load_from_xlsx_bytes(bytes: &[u8], name: &str, locale: &str, tz: &str) -> Result<Workbook, XlsxError>` | https://docs.rs/ironcalc/latest/ironcalc/import/index.html y `…/fn.load_from_xlsx.html`, `…/fn.load_from_xlsx_bytes.html` |
| Guardado | `ironcalc::export::save_to_xlsx(model: &Model<'_>, file_name: &str) -> Result<(), XlsxError>`; también `save_to_icalc` y `save_xlsx_to_writer<W: Write + Seek>(model: &Model<'_>, writer: W) -> Result<W, XlsxError>` | https://docs.rs/ironcalc/latest/ironcalc/export/fn.save_to_xlsx.html y `…/fn.save_xlsx_to_writer.html` |
| `UserModel` | Firmas de todos los métodos usados en §5 (`new_empty`, `from_model`, `from_bytes`, `to_bytes`, `set_user_input`, `get_formatted_cell_value`, `get_cell_content`, `get_cell_style`, `get_cell_type`, `undo`/`redo`, `evaluate`, `pause_evaluation`/`resume_evaluation`, `set_selected_*`, `on_arrow_*`, `on_page_*`, `on_navigate_to_edge_in_direction`, `on_expand_selected_range`, `on_area_selecting`, `set_top_left_visible_cell`, `get_scroll_x/y`, `set_window_width/height`, tamaños, hojas, `resolve_color`, `paste_csv_string`, `set_language`…). `UserModel<'a>` es `Send` y `Sync` | https://docs.rs/ironcalc_base/latest/ironcalc_base/struct.UserModel.html |
| Tipos de estilo | `Style`, `Font`, `Fill { color }`, `Alignment`, `Color { Rgb, Theme, None }`, `Area`, `NavigationDirection`, `SheetProperties`; versión de `ironcalc_base` 0.9.1 | https://docs.rs/ironcalc_base/latest/ironcalc_base/types/struct.Style.html y páginas vecinas; `…/expressions/types/struct.Area.html` |
| Evaluación completa | `Model::evaluate` «Evaluates every formula in the workbook»; `UserModel::set_user_input` llama a `evaluate_if_not_paused` | https://github.com/ironcalc/IronCalc/blob/main/base/src/evaluation.rs y `base/src/user_model/common.rs` (rama `main`; confirmado también en la etiqueta `v0.9.1` el 2026-10-22) |
| Idiomas | `set_language` solo cambia el analizador; con `es`, `=TRUE()` da `#¿NOMBRE?`; locales `en, en-GB, de, es, fr, it` | https://github.com/ironcalc/IronCalc/blob/main/base/src/model.rs, `base/src/test/test_language.rs`, `generate_locale/locales_list.json` |
| Portapapeles | `copy_to_clipboard` usa delimitador tabulador pero `Clipboard.csv` es `pub(crate)`; `paste_csv_string` lee tabulado, una sola lista de cambios | https://github.com/ironcalc/IronCalc/blob/main/base/src/user_model/clipboard.rs |
| `rust_xlsxwriter` | 0.101.0, MIT OR Apache-2.0; `Formula::new` y `set_result`; también existe `Worksheet::set_formula_result`. Por defecto guarda «0» como resultado y marca recalcular al abrir | https://docs.rs/rust_xlsxwriter/latest/rust_xlsxwriter/ , `…/struct.Formula.html` y https://docs.rs/rust_xlsxwriter/0.101.0/rust_xlsxwriter/worksheet/struct.Worksheet.html |
| `calamine` 0.37.0 | MIT, publicada el 2026-10-18; `XlsxCellFormula.value` = «Literal or cached value associated with the cell» y `formula` = texto expandido; `Data::{Int, Float, String, Bool, DateTime, DateTimeIso, DurationIso, Error, Empty}`. Prueba local (calamine 0.37.0 + rust_xlsxwriter 0.101.0): un valor guardado forzado a 7 en `=A1+B1` (real 5) se leyó como `Float(7.0)` con `worksheet_range` y la fórmula como `A1+B1` con `worksheet_formula` | https://crates.io/api/v1/crates/calamine , https://docs.rs/calamine/0.37.0/calamine/struct.XlsxCellFormula.html , `…/struct.XlsxCellReader.html` , `…/enum.Data.html` , https://raw.githubusercontent.com/tafia/calamine/master/src/xlsx/cells_reader.rs |
| Licencia de IronCalc | 0.9.1 (publicada el 2026-10-21): MIT OR Apache-2.0 en crates.io; `LICENSE-MIT` («Copyright 2023 EqualTo GmbH») y `LICENSE-Apache-2.0` presentes en el repositorio | https://crates.io/api/v1/crates/ironcalc , https://raw.githubusercontent.com/ironcalc/IronCalc/main/LICENSE-MIT , `…/LICENSE-Apache-2.0` |
| Productor del archivo | `rust_xlsxwriter` 0.101.0 escribe `<Application>Microsoft Excel</Application>`, `AppVersion` 12.0000 y `calcPr fullCalcOnLoad="1"` (archivo generado y abierto localmente); XlsxWriter (Python) escribe la misma cadena y `AppVersion` 12.0000. Apache POI define `DOCUMENT_CREATOR = "Apache POI"` (no se comprobó que sea lo que escribe en `Application`) | Prueba local; https://raw.githubusercontent.com/jmcnamara/XlsxWriter/main/xlsxwriter/app.py ; https://raw.githubusercontent.com/apache/poi/trunk/poi-ooxml/src/main/java/org/apache/poi/ooxml/POIXMLDocument.java |
| Funciones volátiles de Excel | NOW, TODAY, RAND, RANDARRAY, RANDBETWEEN, OFFSET, INDIRECT; CELL e INFO según sus argumentos (fuente secundaria; no se halló la página de Microsoft) | https://exceljet.net/glossary/volatile-function |
| GPUI: elemento propio e IME | `Element` (`request_layout`, `prepaint`, `paint`), `EntityInputHandler` (11 métodos), `ElementInputHandler`, `window.handle_input`, `shape_line`, `ShapedLine::paint`, `paint_quad`, acciones y `KeyBinding`, portapapeles | https://github.com/zed-industries/zed/blob/main/crates/gpui/examples/input.rs |
| GPUI: ejemplos | `input.rs`, `text.rs`, `text_layout.rs`, `uniform_list.rs`, `scrollable.rs`, `data_table.rs`, `painting.rs`… | https://github.com/zed-industries/zed/tree/main/crates/gpui/examples |
| GPUI: API | `on_scroll_wheel`, `ScrollDelta::{Pixels, Lines}` + `pixel_delta`, `on_mouse_*`, `with_content_mask`, `refresh_interval`, `request_animation_frame`, `prompt_for_paths`, `prompt_for_new_path`, `read_from_clipboard`, `write_to_clipboard` | https://github.com/zed-industries/zed/blob/main/crates/gpui/src/{window.rs,app.rs,interactive.rs,elements/div.rs} |
| GPUI: arranque y dependencias | `gpui_platform::application()`; características; README; macOS necesita `font-kit`; Linux `wayland`/`x11`; Windows sin características; `TestAppContext`; `runtime_shaders` | https://github.com/zed-industries/zed/blob/main/crates/gpui/README.md y `crates/gpui_platform/Cargo.toml` |
| GPUI: licencias internas y parches | Todos los crates internos de la clausura son Apache-2.0; `[patch.crates-io]` con `async-task`, `calloop`, `khronos-egl`… | `Cargo.toml` de cada crate y de la raíz de https://github.com/zed-industries/zed (rama `main`, commit `9e84b066b55bc3609a30dd965075c12255b38b45`) |
| GPUI: dependencias de Linux | Lista de paquetes (`libwayland-dev`, `libxkbcommon-x11-dev`, `libvulkan1`, `libfontconfig-dev`…) | https://github.com/zed-industries/zed/blob/main/script/linux y https://zed.dev/docs/development/linux |
| Tauri: bytes sin JSON | `tauri::ipc::Response::new(body: impl Into<InvokeResponseBody>)`; `InvokeResponseBody::{Json(String), Raw(Vec<u8>)}`; `Channel::send` y `Channel<InvokeResponseBody>`; guía «Returning Array Buffers» | https://docs.rs/tauri/latest/tauri/ipc/struct.Response.html, `…/struct.Channel.html`, `…/enum.InvokeResponseBody.html`, https://v2.tauri.app/develop/calling-rust/ |
| Tauri: lectura en JS | `fetch` al protocolo `ipc`; no-JSON/no-texto → `arrayBuffer()`; caída a `postMessage`; `Channel` de JS con orden por índice | https://github.com/tauri-apps/tauri/blob/dev/crates/tauri/scripts/ipc-protocol.js y `packages/api/src/core.ts` |
| Tauri: hilos | Comandos sin `async` corren en el hilo principal; los `async` en una tarea separada | https://v2.tauri.app/develop/calling-rust/ |
| Tauri: versión, plugins, pruebas | 2.12.5 (21-oct-2026), CLI 2.12.4; `tauri-plugin-dialog` y `tauri-plugin-clipboard-manager` con permisos; `tauri-driver` solo Windows y Linux; `mockIPC` | https://github.com/tauri-apps/tauri/releases , https://v2.tauri.app/plugin/dialog/ , https://v2.tauri.app/plugin/clipboard/ , https://v2.tauri.app/develop/tests/webdriver/ , https://v2.tauri.app/develop/tests/mocking/ |
| Tauri: requisitos del sistema | Lista de `apt` para WebKitGTK; WebView2 y MSVC en Windows; Xcode en macOS | https://v2.tauri.app/start/prerequisites/ |
| Ejecutores de GitHub | `ubuntu-24.04`, `windows-2025`, `macos-26` (arm64) y `macos-latest` = macOS 26 arm64; sin GPU documentada | https://github.com/actions/runner-images/blob/main/README.md y https://docs.github.com/en/actions/reference/runners/github-hosted-runners |
| PresentMon | Opciones `--process_name`, `--process_id`, `--output_file`, `--timed`, `--terminate_after_timed`, `--qpc_time`; columnas `MsBetweenPresents`, `MsBetweenDisplayChange`, `DisplayLatency`, `DisplayedTime`; grupo «Performance Log Users» | https://github.com/GameTechDev/PresentMon/blob/main/README-ConsoleApplication.md y `README.md` |
| ffmpeg | `avfoundation` (`-framerate`, `-capture_cursor`), `x11grab`, `kmsgrab`; filtros `ddagrab` y `gfxcapture` | https://ffmpeg.org/ffmpeg-devices.html y https://ffmpeg.org/ffmpeg-filters.html |
| MangoHud | Opciones de registro `output_folder`, `log_duration`, `autostart_log`, `log_interval` | Resultados de búsqueda que citan el README (git.blob42.xyz/Archives/MangoHud y otros espejos); **no** se leyó el repositorio oficial |
| Locales (este Mac) | `x2t` 10.0.0.169 presente y con uso `x2t in out`; plantillas de `xctrace`; `footprint` con `-a`, `-j`; ONLYOFFICE con `editors_helper`, `editors_helper (GPU)`, `editors_helper (Renderer)`; pantalla 2560×1600 Retina; hay procesos `com.apple.WebKit.*` de otras apps | Comandos ejecutados en el Mac del usuario |

**No se pudo verificar** (queda como `[NO VERIFICADO]` o `[A VERIFICAR EN Txxx]`): el valor exacto que escriben Apache POI y LibreOffice en `Application`; si todo archivo de A4 trae `docProps/app.xml`; cómo distingue `calamine` una fórmula sin resultado guardado de un resultado de texto vacío y cómo devuelve el número de serie de una fecha; qué parte del valor sin formato y de la lista de funciones reconocidas expone IronCalc 0.9.1 (T115); la licencia de `zip`; comportamiento de IronCalc **0.9.1** frente a `main` en idiomas y portapapeles (la evaluación completa sí quedó confirmada en 0.9.1); unidad y base de los tamaños de fila/columna; formato del argumento de `on_expand_selected_range`; campos de `SelectedView`; API de Apple (ScreenCaptureKit; la página no cargó); `purge`, RAMMap, `drop_caches`; permisos TCC de macOS; UIPI de Windows; XTest en Wayland; atribución de procesos de memoria en las tres plataformas; límites de altura de elemento por motor; orden de eventos de composición en Safari/WebView2/WebKitGTK; calidad del IME de GPUI en Windows y Linux; requisitos de compilación de GPUI en Windows; cómo ONLYOFFICE abre un archivo recibido como argumento y su ruta de instalación en Windows y Linux. (Ctrl+U como edición en Excel para Mac quedó confirmado: https://exceljet.net/shortcuts/edit-the-active-cell .) Las fuentes de contexto (ONLYOFFICE, Zed, WebKit, testimonios) están en `research.md`.
