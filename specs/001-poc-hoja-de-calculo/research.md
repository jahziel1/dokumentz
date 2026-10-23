# Investigación 001 — Decisiones técnicas de la PoC de hoja de cálculo

**Fecha:** 2026-10-22 · **Estado:** Borrador · **Spec:** [spec.md](spec.md) · **Constitución:** [constitucion.md](../constitucion.md)

Este documento reúne las decisiones técnicas ya tomadas por el usuario o derivadas de ellas, con su porqué, las alternativas consideradas y las fuentes. El CÓMO detallado vive en `plan.md`.

**Convenciones**
- Los hechos llevan fuente (URL). Lo que **no** se pudo verificar se marca como *sin verificar*.
- Una decisión «cerrada» fue tomada por el usuario; una decisión «propuesta» es una consecuencia técnica que el plan puede ajustar con datos (Constitución, principio VIII).
- Versiones y fechas son las vigentes el 2026-10-22.

## Índice

| ID | Decisión | Estado |
|---|---|---|
| D-01 | Núcleo en Rust sin dependencias de interfaz | Cerrada |
| D-02 | IronCalc como motor de hoja de cálculo | Cerrada |
| D-03 | Dos interfaces: Tauri 2 (webview del sistema) y GPUI | Cerrada |
| D-04 | No hacer fork de ONLYOFFICE | Cerrada |
| D-05 | No usar LibreOfficeKit en esta PoC | Cerrada (para la PoC) |
| D-06 | `.xlsx` como formato nativo | Cerrada |
| D-07 | Entrada de texto e IME: textarea oculto (Tauri) y manejo de entrada propio (GPUI) | Propuesta |
| D-08 | Riesgos de la webview por plataforma | Riesgo documentado |
| D-09 | Costo del IPC en Tauri y cómo evitarlo | Propuesta |
| D-10 | Tauri 3 con CEF como salida de emergencia | Documentada, no ejecutada |
| D-11 | Método de medición justo con ONLYOFFICE | Principios aquí; detalle en `plan.md` |
| D-12 | Línea base de ONLYOFFICE inspeccionada en el Mac del usuario | Datos |
| D-13 | Rendimiento en hardware real; CI y máquinas virtuales solo funcional | Cerrada |
| D-14 | Diseño visual propio, no un calco de Office | Cerrada |
| D-15 | Licencias de las dependencias principales | Verificación pendiente en el plan (IronCalc, `calamine` y `rust_xlsxwriter` ya verificados) |
| D-16 | Arquitectura de la suite; IronCalc como pieza y no como «el motor» | Cerrada (desarrollada en `../000-arquitectura-suite/arquitectura.md`) |
| D-17 | Método de compatibilidad con los valores que Excel guardó en el archivo (`calamine`) | Propuesta (los umbrales de SC-007 se adoptan por defecto, como los de SC-001…SC-006) |
| D-18 | Nombres de producto en lugar de `poc-*` | Cerrada |

---

## D-01 — Núcleo en Rust sin dependencias de interfaz

**Decisión.** Todo lo que no es dibujar ni recibir entradas (modelo de documento, fórmulas, formato de números, lectura y escritura de archivos, selección y navegación, deshacer) vive en un crate de núcleo en Rust que no depende de ninguna interfaz. Las dos interfaces lo consumen. El núcleo expone una «lista de dibujo» del área visible (celdas con texto formateado, estilo y geometría) y comandos de edición y navegación.

**Por qué.**
- Es la regla de oro (Constitución, principio III): permite tener dos interfaces sobre el mismo núcleo y compararlas con justicia, y cambiar de interfaz sin reescribir lo valioso.
- Rust no tiene recolector de basura. Zed explicó que dejó la web como base de su editor por el presupuesto de ~8,33 ms por cuadro a 120 Hz, las pausas de recolección de basura y los relayouts del DOM.
- La línea base se comporta al revés: los editores de ONLYOFFICE son JavaScript (`cell/sdk-all.js` de 10 MB) ejecutado dentro de un navegador embebido (ver D-12). Google Sheets ganó ~2× al cambiar el motor de JS a WasmGC, lo que indica que el tiempo de ejecución del motor pesa.

**Alternativas consideradas.**
- *Motor en JavaScript/TypeScript dentro de la webview* (como ONLYOFFICE): descartada, ata el núcleo a una sola interfaz y reproduce la arquitectura que se quiere superar.
- *Motor compilado a WebAssembly dentro de la webview*: descartada para la PoC; no sirve a la interfaz nativa, duplicaría arquitecturas, y la versión web/WASM está fuera de alcance.
- *Núcleo en C++* (como `core` de ONLYOFFICE, AGPL-3.0): descartada por licencia (D-04) y porque el usuario ya fijó Rust.

**Fuentes.**
- Presupuesto por cuadro, recolección de basura y relayouts: https://zed.dev/blog/videogame
- Google Sheets con WasmGC: https://web.dev/case-studies/google-sheets-wasmgc
- Tamaño de los editores de ONLYOFFICE: inspección local (D-12).

---

## D-02 — IronCalc como motor de hoja de cálculo

**Decisión.** Se usan los crates `ironcalc` y `ironcalc_base` (versión **0.9.1**, publicada el 21-oct-2026) envueltos por el núcleo. No se escribe un motor de fórmulas propio. Se fija la versión exacta.

**Por qué.**
- Está escrito en Rust y trae lógica de interfaz ya hecha en `ironcalc_base::UserModel`: `set_user_input`, `get_formatted_cell_value`, `get_cell_content`, `get_cell_style`, `undo`/`redo`, `evaluate`, `pause_evaluation`/`resume_evaluation`, selección y navegación (`set_selected_cell`, `set_selected_range`, `on_arrow_*`, `on_page_up/down`, `on_navigate_to_edge_in_direction`, `on_expand_selected_range`, `set_top_left_visible_cell`, `get_scroll_x/y`), tamaños (`get_column_width`, `get_row_height`…), hojas (`new_sheet`, `rename_sheet`, `get_worksheets_properties`…), `formula_completion`, `to_bytes`/`from_bytes` y `flush_send_queue`/`apply_external_diffs`.
- Idiomas de funciones: en, es, fr, de, it, e incluye `BUSCARV`, `BUSCARX`, `SI.ERROR` y `CONTAR.SI` (`generate_language/src/languages.json`), lo que cubre FR-006.
- Lee y escribe `.xlsx`: el módulo `ironcalc::import` ofrece `load_from_xlsx`, `load_from_xlsx_bytes` y `load_from_icalc`; `ironcalc::export::save_to_xlsx(&model, path)` guarda. `Model::new_empty(name, locale, timezone, language)` crea un libro.
- Licencia permisiva: **MIT OR Apache-2.0**, verificada en crates.io (0.9.1) y en los archivos `LICENSE-MIT` y `LICENSE-Apache-2.0` del repositorio (D-15). Financiado por NLnet.

**Riesgos que la PoC mide o vigila.**
- Es **pre-1.0**: la API puede cambiar; por eso se fija la versión.
- **Reevalúa todo el libro en cada edición** (verificado en el código de v0.9.1: `UserModel::set_user_input` llama a `evaluate_if_not_paused`, que llama a `Model::evaluate`, y cada pasada de `run_pass` recorre todas las posiciones con fórmula). No es incremental: el costo de M-05 crece con el número de fórmulas del libro y no solo con las dependientes. `pause_evaluation`/`resume_evaluation` permite agrupar ediciones. M-05 lo mide.
- Es **la calculadora de la hoja, no el motor de la suite** (D-16). Cuántas de sus funciones calculan igual que Excel no se sabe: lo mide SC-007 (D-17).
- La fidelidad de ida y vuelta del `.xlsx` se mide con SC-005.
- La discrepancia de licencia entre GitHub y el README quedó resuelta: es doble MIT OR Apache-2.0 (D-15). Falta auditar las licencias de las dependencias transitivas con `cargo deny` en CI.

**Alternativas consideradas.**
- *Motor propio*: descartada, es trabajo enorme y no responde las preguntas de la PoC.
- *Motor de ONLYOFFICE* (`core`, `sdkjs`): descartada por licencia AGPL-3.0 (D-04).
- *Motor de LibreOffice*: descartada para la PoC (D-05).
- *Otros motores de hojas en Rust*: no se evaluaron en esta investigación.

**Fuentes.**
- Repositorio: https://github.com/ironcalc/IronCalc
- `UserModel`: https://docs.rs/ironcalc_base/latest/ironcalc_base/struct.UserModel.html
- Módulo `import` (versión 0.9.1 y nombres de funciones, verificado el 2026-10-22): https://docs.rs/ironcalc/latest/ironcalc/import/index.html
- Idiomas: `generate_language/src/languages.json` en el repositorio.
- Evaluación completa en cada edición (v0.9.1): https://github.com/ironcalc/IronCalc/blob/v0.9.1/base/src/evaluation.rs (`Model::evaluate`, `run_pass`) y https://github.com/ironcalc/IronCalc/blob/v0.9.1/base/src/user_model/common.rs (`evaluate_if_not_paused`).

---

## D-03 — Dos interfaces: Tauri 2 (webview del sistema) y GPUI

**Decisión.** Se construyen **dos** interfaces con exactamente los mismos requisitos (FR-001…FR-011) sobre el mismo núcleo y se miden las dos. La PoC decide cuál queda con la regla de decisión de la spec (sección 9).
- **Interfaz A: Tauri 2 estable** (2.12.5 a 2026-10-21) con la **webview del sistema**; frontend mínimo en TypeScript sin framework pesado y dibujo en canvas 2D.
- **Interfaz B: GPUI**, nativa con GPU, como dependencia **git fijada a un commit** de Zed.

**Por qué.**
- La pregunta «¿webview o nativa?» no se resuelve con argumentos; se mide. El usuario quiere ver los datos en macOS, Windows y Linux.
- *A favor de Tauri:* menos trabajo de interfaz y más fácil lograr el aspecto de Office/ONLYOFFICE con HTML/CSS en la fase 1. Hay un antecedente de una suite que usa la webview del sistema sobre un motor ofimático: Collabora Office de escritorio (nov-2025), sobre el motor de LibreOffice.
- *En contra de Tauri:* la dependencia de la webview de cada sistema (D-08) y el costo del IPC (D-09). Porsche necesitó CSS por plataforma y dejó Tauri en pausa; Hopp (ene-2026) listó problemas de WebKit y mueve ventanas críticas a Rust nativo (iced).
- *A favor de GPUI:* licencia Apache-2.0 (`crates/gpui/Cargo.toml`: `license = "Apache-2.0"`), Zed 1.0 (29-abr-2026) corre en macOS, Windows y Linux y su diseño parte de un presupuesto de ~8,33 ms por cuadro. En la PoC la cuadrícula es dibujo propio, así que no hace falta un editor de texto enriquecido.
- *En contra de GPUI:* la versión en crates.io (0.2.2) es del 22-oct-2025 y `publish = true`, por lo que se usa dependencia git fijada a un commit. No se encontró un editor de texto enriquecido maduro en interfaces nativas de Rust: es un riesgo para el procesador de textos (fase 3), no para esta PoC. Su manejo de entrada (D-07) y su calidad por plataforma deben probarse.

**Alternativas consideradas.**
- *Solo Tauri* o *solo GPUI*: descartadas, no permiten comparar.
- *Tauri 3 con CEF (Chromium incluido)*: reservada como salida de emergencia (D-10).
- *iced* (a donde Hopp mueve ventanas críticas) y otros marcos nativos de Rust: no se evaluaron en esta PoC.
- *Electron u otro marco con navegador incluido*: no se evaluó; la línea base ONLYOFFICE (CEF) ya representa la familia de «navegador incluido» (D-12).

**Fuentes.**
- Versiones de Tauri y alfa de Tauri 3: https://github.com/tauri-apps/tauri/releases
- GPUI (licencia, `publish`): `crates/gpui/Cargo.toml` en https://github.com/zed-industries/zed
- Zed 1.0 en las tres plataformas: https://zed.dev/blog/zed-1-0
- Por qué Zed dejó la web: https://zed.dev/blog/videogame
- Collabora Office de escritorio: https://www.collaboraonline.com/blog/collabora-online-now-available-on-desktop/
- Porsche: https://opensource.porsche.com/porschedigital-technology-radar/tools/tauri/
- Hopp: https://gethopp.app/blog/hate-webkit

---

## D-04 — No hacer fork de ONLYOFFICE

**Decisión.** No se parte de un fork de ONLYOFFICE ni se copia su código. ONLYOFFICE se usa como **referencia de comportamiento**, como **línea base de rendimiento** y como **juez de compatibilidad** (Constitución, principio VII).

**Por qué.**
- **Licencia.** `sdkjs`, `web-apps` y `core` son AGPL-3.0, y hay condiciones adicionales sobre el logo que están en disputa con Euro-Office; la FSF sostiene que ONLYOFFICE no puede usar la AGPL para restringir forks. Un fork heredaría una licencia y un conflicto sin resolver, y la licencia del proyecto aún está pendiente (D-15).
- **Arquitectura.** El objetivo es ser más rápido que ONLYOFFICE y su arquitectura (editores en JavaScript sobre un navegador embebido, 941 MB; D-12) es justamente la línea base. Un fork empezaría por heredarla.
- **Percepción.** Usuarios del foro dicen que sigue siendo lento frente a otras suites (evidencia anecdótica, no una medición; la PoC la sustituye con M-01…M-08).

**Alternativas consideradas.**
- *Fork completo:* descartada por licencia y arquitectura.
- *Aportar mejoras de rendimiento a ONLYOFFICE:* descartada, no es el proyecto que el usuario quiere.
- *Usarlo solo como referencia, línea base y juez:* **adoptada**.

**Fuentes.**
- Repositorios AGPL-3.0: https://github.com/ONLYOFFICE/sdkjs · https://github.com/ONLYOFFICE/web-apps · https://github.com/ONLYOFFICE/core
- Disputa de licencia (Euro-Office, FSF): https://linuxiac.com/fsf-says-onlyoffice-cannot-use-agpl-to-restrict-forks/
- Foro: https://forum.onlyoffice.com/t/fast-but-still-not-fast-enough/10209

---

## D-05 — No usar LibreOfficeKit en esta PoC

**Decisión.** La PoC no usa LibreOfficeKit (la API de incrustación de LibreOffice) como núcleo. LibreOffice se usa como referencia de comportamiento y como juez de compatibilidad.

**Por qué.**
- La PoC prueba una hipótesis concreta: un **núcleo ligero en Rust con interfaz delgada** (preguntas 1 y 3 de la spec). LibreOfficeKit pondría el motor de LibreOffice como núcleo; sería otra hipótesis y no responde si la regla de oro funciona con un núcleo propio.
- LibreOfficeKit es la API para acceder a LibreOffice desde C/C++ sin UNO; su documentación describe una API de renderizado por teselas **experimental e inestable** (requiere definir `LOK_USE_UNSTABLE_API`), con salida en un búfer de mapa de bits BGRA de 32 bits. Dibujar eso en dos interfaces distintas añade una capa que no es parte de lo que se quiere medir.
- No se descarta para el futuro. Como no se encontró un editor de texto enriquecido maduro en interfaces nativas de Rust, el procesador de textos (fase 3) puede replantearlo. Precedente: Collabora Office de escritorio (nov-2025) usa la webview del sistema sobre el motor de LibreOffice. Re-evaluar con datos antes de la fase 3.

**Alternativas consideradas.**
- *LibreOfficeKit como núcleo (con las dos interfaces como visores de teselas):* aplazada.
- *LibreOffice solo como juez de compatibilidad:* **adoptada**.

**Fuentes.**
- Documentación de LibreOfficeKit (verificada el 2026-10-22): https://docs.libreoffice.org/libreofficekit.html
- Collabora Office de escritorio: https://www.collaboraonline.com/blog/collabora-online-now-available-on-desktop/

---

## D-06 — `.xlsx` como formato nativo

**Decisión.** El formato nativo de la hoja de cálculo es `.xlsx`. «Guardar» escribe `.xlsx` directamente, sin formato intermedio propio como formato principal. La PoC garantiza ida y vuelta de **valores y fórmulas** (FR-011, SC-005); estilos y objetos no soportados quedan fuera de la garantía.

**Por qué.**
- Constitución, principio IV: guardar nunca debe sentirse como «convertir»; las personas intercambian archivos de Office con otros programas.
- El motor ya importa y exporta `.xlsx` (D-02), así que la PoC puede medir M-07 y verificar compatibilidad sin código extra.
- ONLYOFFICE y Excel actúan como jueces: abrir el resultado sin errores ni avisos de reparación.

**Riesgos.** Contenido fuera de alcance (gráficos, celdas combinadas, formato condicional…) puede perderse al guardar; la spec lo deja como aclaración pendiente.

**Alternativas consideradas.**
- *Formato propio con exportación a `.xlsx`:* descartada por el principio IV (guardar sería «convertir»).
- *Formato OpenDocument como nativo:* descartada; la referencia de uso son los formatos de Office. No se evaluó con datos.

**Fuentes.** Decisión de producto del usuario (visión del proyecto) y capacidades del motor (D-02).

---

## D-07 — Entrada de texto e IME

**Decisión (propuesta).**
- **Tauri:** un **textarea oculto** colocado sobre la celda activa recibe la entrada del teclado, las teclas muertas, el IME y el dictado; el dibujo de la cuadrícula en canvas no implementa IME. Se envían al núcleo el texto confirmado y los eventos de composición.
- **GPUI:** se usa el **manejo de entrada propio de GPUI**: el editor de celda implementa el trait `EntityInputHandler`, envuelto en `ElementInputHandler` y asociado durante el dibujo con `Window::handle_input`.

**Por qué.**
- **EditContext** (la API moderna de entrada de texto de la web) **no existe en Safari ni en Firefox**, así que no puede ser la base en macOS (WKWebView) ni en Linux (WebKitGTK). Se necesita una ruta que funcione igual en las tres plataformas: el textarea oculto, patrón descrito como el usado por Google Docs y ONLYOFFICE (*sin verificar con fuente primaria en esta investigación*; no condiciona la decisión, que se apoya en la falta de EditContext).
- En GPUI, la documentación de `EntityInputHandler` indica que sirve para «manejar entrada textual al implementar un editor, un campo, etc.» e incluye métodos de texto marcado (`marked_text_range`, `replace_and_mark_text_in_range`, `unmark_text`), los que se usan para la composición de IME. La página no usa el término «IME» (verificado el 2026-10-22).

**Riesgos que la PoC prueba (SC-004).**
- Desplazar la vista con una composición abierta (caso límite de la spec) y que el textarea se quede fuera del área visible.
- Posición de la ventana de candidatos del IME junto a la celda.
- Dictado del sistema en macOS en ambas interfaces.
- La calidad real del IME de GPUI en Windows y Linux no está verificada: Zed 1.0 corre en las tres plataformas, pero eso no demuestra IME correcto en cada una.

**Alternativas consideradas.**
- *`contenteditable` en cada celda:* descartada, exige un elemento DOM por celda (D-09).
- *Implementar el IME a mano sobre el canvas, sin textarea:* descartada, sin EditContext la web no ofrece una vía de entrada de composición sin un elemento editable.
- *Solo eventos de teclado:* descartada, no cumple FR-007.
- *EditContext donde exista:* descartada para la PoC; tener una sola ruta en las tres plataformas simplifica las pruebas de SC-004. No se verificó su disponibilidad en WebView2.

**Fuentes.**
- Compatibilidad de EditContext: https://caniuse.com/mdn-api_editcontext
- Entrada en GPUI (`EntityInputHandler`, verificado el 2026-10-22): https://docs.rs/gpui/latest/gpui/trait.EntityInputHandler.html

---

## D-08 — Riesgos de la webview por plataforma

**Decisión.** La PoC mide Tauri con la **webview del sistema** en hardware real de cada plataforma y registra en cada informe la versión del motor web y la configuración gráfica usada.

| Plataforma | Motor web | Riesgo | Mitigación / acción |
|---|---|---|---|
| macOS | WKWebView | La versión de WebKit depende de la versión de macOS del usuario. | Registrar la versión de macOS y de WebKit en el informe; solo se prueba en macOS 26.6.2. |
| Windows | WebView2 (evergreen) | Se actualiza solo: el resultado puede cambiar entre mediciones. Existe la opción de fijar una versión. | Registrar la versión del runtime; valorar versión fija para reproducibilidad. |
| Linux | WebKitGTK según la distribución | Con NVIDIA: ventana en blanco, parpadeo, cierres; WebGL y canvas pueden caer a un modo lento **sin aviso**. | Variables de entorno (por ejemplo `WEBKIT_DISABLE_DMABUF_RENDERER=1`), que **no deben aplicarse a todos**; medir en Linux con NVIDIA y registrar las mitigaciones usadas. |

**Por qué.** Los casos de Porsche (CSS por plataforma, Tauri en pausa) y de Hopp (problemas de WebKit, ventanas críticas a Rust nativo) muestran que la diferencia entre motores web es un riesgo real. Por eso la regla de decisión da peso especial a M-03 y a Linux con NVIDIA.

**Alternativas consideradas.**
- *Aplicar mitigaciones globales:* descartada, no deben aplicarse a todos.
- *Tauri con CEF:* D-10.
- *Medir solo en macOS:* descartada, deja sin respuesta la pregunta 4 de la spec.

**Fuentes.**
- Versiones de webview por plataforma: https://v2.tauri.app/reference/webview-versions/
- Linux y gráficos (actualizado el 15-jun-2026): https://v2.tauri.app/develop/debug/linux-graphics/
- WebView2 (evergreen y versión fija): https://learn.microsoft.com/en-IE/microsoft-edge/webview2/concepts/developer-guide
- Porsche: https://opensource.porsche.com/porschedigital-technology-radar/tools/tauri/
- Hopp: https://gethopp.app/blog/hate-webkit

---

## D-09 — Costo del IPC en Tauri y cómo evitarlo

**Decisión (propuesta).** En cada cuadro de desplazamiento **no** se piden celdas por JSON. El diseño evita el costo del puente entre Rust y la webview con: respuesta **binaria** de Tauri 2, búfer de datos del área visible **con margen y precarga**, y dibujo en **canvas 2D** desde ese búfer. El costo del puente se mide en M-03 y con la instrumentación interna.

**Por qué.**
- La documentación de Tauri advierte que serializar a JSON un resultado grande puede ralentizar la aplicación y recomienda devolver datos binarios con `tauri::ipc::Response` (sección «Returning Array Buffers», verificada el 2026-10-22).
- En hojas de cálculo web no se usa un elemento DOM por celda: se dibuja en canvas o WebGL solo lo visible.
- Los navegadores limitan la altura de los elementos, por lo que con cientos de miles de filas hay que escalar el desplazamiento virtual.

**Alternativas consideradas.**
- *JSON por cuadro:* descartada por la advertencia de la documentación.
- *Un elemento DOM por celda o filas virtualizadas en DOM:* descartada para la cuadrícula, por los motivos de Quadratic.
- *WebGL en lugar de canvas 2D:* es lo que eligió Quadratic. Se considera solo si el canvas 2D no cumple M-03; la PoC empieza con canvas 2D.
- *Núcleo en WebAssembly dentro de la webview para evitar el IPC:* descartada (D-01).

**Fuentes.**
- Devolver búferes en Tauri: https://v2.tauri.app/develop/calling-rust/
- Hoja de cálculo con WebGL (Quadratic): https://www.quadratichq.com/blog/building-a-high-performance-spreadsheet-renderer-why-we-chose-webgl-over-html
- Límite de altura de elementos y escalado del desplazamiento (Deephaven): https://deephaven.io/blog/2022/01/24/displaying-a-quadrillion-rows/

---

## D-10 — Tauri 3 con CEF como salida de emergencia

**Decisión.** La PoC usa **Tauri 2 estable con la webview del sistema**, que es lo que se quiere medir. **Tauri 3 con CEF queda documentada como salida de emergencia**: no se ejecuta en esta PoC y solo se evalúa si los datos muestran que la webview del sistema falla en una plataforma (sobre todo Linux con NVIDIA) y no hay alternativa suficiente.

**Por qué.**
- `3.0.0-alpha.5` (14-oct-2026) incluye `tauri-runtime-cef`, que permite incluir Chromium como alternativa a la webview del sistema. Es **alfa**: sin garantías de estabilidad.
- Con CEF el motor web sería el mismo en las tres plataformas, lo que elimina la variabilidad de D-08.
- Costo previsible (*hipótesis, a medir si se llega a usar*): llevar Chromium con la aplicación aumenta M-08 y probablemente M-06. ONLYOFFICE ya usa Chromium Embedded Framework (D-12), de modo que una opción con CEF se parecería a la arquitectura de la línea base y no ganaría por sí sola en esas medidas.

**Alternativas consideradas.**
- *Pasar a GPUI:* es la salida prevista por la regla de decisión (spec, sección 9).
- *Mitigaciones de variables de entorno en Linux:* D-08.

**Fuentes.** https://github.com/tauri-apps/tauri/releases

---

## D-11 — Método de medición justo con ONLYOFFICE

**Decisión (principios; el detalle va en `plan.md`).** Las tres aplicaciones (PoC-Tauri, PoC-GPUI, ONLYOFFICE 10.0.0) se miden con el **mismo método externo** (por ejemplo, grabación de pantalla a alta tasa de cuadros con marcas de tiempo, o automatización del sistema), y las dos PoC además con su **instrumentación interna** (FR-012). Cada medida: 5 repeticiones, mediana y p95, misma máquina, en frío y en caliente cuando aplique.

**Por qué.** ONLYOFFICE no se puede instrumentar por dentro, así que solo hay comparación justa con una medición externa idéntica. Las medidas internas diagnostican, pero **no se mezclan** con las externas.

**Reglas para que sea justo** (a concretar en `plan.md`):
- Mismo equipo, mismo archivo (A1–A3 generados con semilla fija), mismas operaciones, misma pantalla y misma frecuencia de refresco declarada.
- Misma definición de «ventana usable» y de «primera pantalla con datos» para las tres aplicaciones; en ONLYOFFICE se declara si el arranque termina en la pantalla de inicio o en el editor de hojas.
- M-06 suma **todos** los procesos de cada aplicación (en ONLYOFFICE, también los auxiliares de GPU y renderizado de CEF; en Tauri, los de la webview) con la misma métrica de memoria.
- ONLYOFFICE en su versión fija (10.0.0 en macOS) y su configuración por defecto; cualquier ajuste se documenta.
- ONLYOFFICE tiene más funciones que la PoC (procesador de textos, presentaciones, cinta): el alcance distinto se declara en el informe, **sin corregir** los datos.
- El sobrecosto del propio método de captura se mide o se aplica igual a las tres aplicaciones.
- Se reportan también las medidas en las que la PoC pierde.
- Las tablas son por plataforma; no se extrapola de una plataforma a otra.

**Alternativas consideradas.**
- *Solo instrumentación interna:* descartada, no es comparable con ONLYOFFICE.
- *Cronómetro manual:* descartada, demasiado ruido para latencias de decenas de milisegundos.
- *Benchmarks sintéticos sin la aplicación completa:* descartada, la pregunta 1 es sobre cómo se siente la aplicación.

**Fuentes.** Sin fuente externa: decisión metodológica del proyecto (Constitución, principio VIII). Datos de ONLYOFFICE: D-12.

---

## D-12 — Línea base de ONLYOFFICE inspeccionada en el Mac del usuario

**Datos (inspección local previa a esta investigación; no es una medición de rendimiento).**

| Dato | Valor |
|---|---|
| Versión | ONLYOFFICE 10.0.0 |
| Binario | Nativo arm64 |
| Tamaño de la app | 941 MB |
| Motor web | `Chromium Embedded Framework.framework`, con procesos auxiliares GPU y Renderer |
| Editores | JavaScript en `Contents/Resources/editors/sdkjs` |
| `word/sdk-all.js` | 8,9 MB |
| `cell/sdk-all.js` | 10 MB |
| `slide/sdk-all.js` | 9,1 MB |
| WebAssembly | Solo para piezas sueltas: `fonts.wasm` 3,4 MB, `drawingfile.wasm` 10 MB (PDF), `spell.wasm`, `zlib.wasm`, hash |
| Máquina | Apple M1, 16 GB de RAM, macOS 26.6.2 |

**Implicaciones.**
- La línea base es una aplicación de **JavaScript sobre Chromium incluido**: M-06 debe sumar los procesos auxiliares y M-08 compara contra 941 MB, que incluyen procesador de textos y presentaciones. El alcance distinto se declara (D-11).
- ONLYOFFICE usa WebAssembly solo en piezas sueltas; el motor de la hoja de cálculo es JavaScript.
- Aún **no** hay mediciones de ONLYOFFICE para M-01…M-07: se obtienen en la PoC.
- No se inspeccionó ONLYOFFICE en Windows ni en Linux (aclaración 3 de la spec).

**Fuentes.** Inspección local del paquete de la aplicación. Repositorios AGPL-3.0 (https://github.com/ONLYOFFICE/sdkjs, https://github.com/ONLYOFFICE/web-apps, https://github.com/ONLYOFFICE/core) y foro (https://forum.onlyoffice.com/t/fast-but-still-not-fast-enough/10209) como contexto.

---

## D-13 — Rendimiento en hardware real; CI y máquinas virtuales solo funcional

**Decisión.** El rendimiento se mide solo en **hardware real** de macOS, Windows y Linux (idealmente Linux con NVIDIA). La integración continua (GitHub Actions, tres plataformas) compila y ejecuta pruebas funcionales; **no** mide rendimiento (SC-006). Las máquinas virtuales sirven para pruebas funcionales.

**Por qué.** Las VMs no tienen GPU, y el problema que se quiere detectar (D-08, NVIDIA, caída a modo lento de canvas/WebGL) depende de la GPU real. El usuario **solo tiene Mac**, así que hace falta conseguir hardware real; es una aclaración pendiente (spec, aclaración 2).

**Alternativas consideradas.**
- *Medir en VM:* descartada por falta de GPU.
- *No medir Windows y Linux:* descartada, dejaría la pregunta 4 sin responder.
- *Alquilar máquinas con GPU en la nube:* no evaluada; depende de la aclaración de hardware.

**Fuentes.** Información del usuario sobre su equipo y la regla de decisión; Linux y NVIDIA: https://v2.tauri.app/develop/debug/linux-graphics/

---

## D-14 — Diseño visual propio, no un calco de Office

**Decisión.** Estructura, nombres, atajos de cada sistema y funciones en el idioma del usuario se parecen a Office; **íconos y estilo visual son propios** (Constitución, principio V). En la PoC no hay cinta; antes de la fase 1 se revisa su diseño con asesoría legal.

**Por qué (riesgo legal; los datos son orientativos y no son asesoría legal).**
- Microsoft anunció en 2007 una licencia de la interfaz de Office (cinta) que excluía los productos que compiten directamente con Word, Excel, PowerPoint, Outlook y Access. Un resumen posterior del texto de la licencia define como excluido el software que cumple primordialmente las mismas funciones generales y se crea o comercializa como reemplazo. Los términos vigentes en 2026 **no se verificaron**.
- En *Microsoft v. Corel*, un jurado dictaminó en febrero de 2018 que Corel infringió a propósito seis patentes de interfaz de Office (dos de utilidad y cuatro de diseño) y otorgó 278 000 USD a Microsoft. La sentencia final y las apelaciones **no se verificaron**.

**Alternativas consideradas.**
- *Calco de la cinta de Office:* descartada por riesgo legal.
- *Interfaz sin parecido con Office:* descartada, va contra el principio de «sentirse como Office».

**Fuentes.**
- Licencia Fluent UI (2007): https://visualstudiomagazine.com/blogs/desmond-file/2007/01/microsoft-gets-fluent.aspx · https://www.itwriting.com/blog/885-developers-will-you-or-have-you-signed-for-ribbon-rights.html
- *Microsoft v. Corel* (veredicto): https://ipwatchdog.com/2018/02/13/93710/ · https://patentlawcenter.pli.edu/?p=10843 · https://office-watch.com/2018/microsoft-wins-office-patent-battle-corel/

---

## D-15 — Licencias de las dependencias principales

**Estado.** La licencia del **proyecto** quedó decidida el 2026-10-22: **MIT OR Apache-2.0** (Constitución v1.1.0, principio I). Todas las dependencias principales de abajo son compatibles; se prohíben las dependencias GPL, LGPL y AGPL.

**Corpus públicos para A4 (verificado el 2026-10-22).** Apache POI, `test-data/spreadsheet` en https://github.com/apache/poi (licencia Apache-2.0 en `legal/LICENSE`): 353 `.xlsx` y 14 `.xlsm` (por ejemplo `SimpleMacro.xlsm`, `xlmmacro.xlsm`). LibreOffice, `sc/qa/unit/data/xlsx` en https://github.com/LibreOffice/core (MPL-2.0): 312 `.xlsx`. Se usan solo en local y no se redistribuyen en el repositorio. SheetJS `test_files` no está accesible en GitHub (acceso bloqueado).

| Dependencia | Licencia según las fuentes consultadas | Verificación pendiente |
|---|---|---|
| IronCalc 0.9.1 | **MIT OR Apache-2.0.** Verificado el 2026-10-22: crates.io lo declara para la versión 0.9.1 (publicada el 2026-10-21), y el repositorio contiene `LICENSE-MIT` (MIT, «Copyright 2023 EqualTo GmbH») y `LICENSE-Apache-2.0` (Apache License 2.0). La discrepancia previa (GitHub mostraba solo Apache-2.0) queda resuelta | Auditar las licencias de las dependencias transitivas (`cargo deny check licenses` en CI) |
| `calamine` 0.37.0 | **MIT** (crates.io, publicada el 2026-10-18; el README también indica MIT) | Auditar sus dependencias transitivas con `cargo deny` |
| `rust_xlsxwriter` 0.101.0 | **MIT OR Apache-2.0** (docs.rs, 2026-10-22). Solo para archivos de prueba de `tools/compat-check` y del generador | Auditar sus dependencias transitivas con `cargo deny` |
| GPUI | Apache-2.0 (`license = "Apache-2.0"` en `crates/gpui/Cargo.toml`) | El repositorio de Zed es, en su conjunto, principalmente GPL-3.0-or-later «con componentes Apache-2.0 donde se indica». **Verificado el 2026-10-22 en la rama principal:** todas las dependencias internas directas de `gpui` dentro del espacio de trabajo de Zed son Apache-2.0 (`bench_metrics`, `collections`, `gpui_macros`, `gpui_platform`, `gpui_shared_string`, `gpui_util`, `http_client`, `path`, `refineable`, `reqwest_client`, `scheduler`, `sum_tree`, `util_macros`). Falta repetir la auditoría **transitiva** en el commit fijado (con `cargo deny check licenses` en CI) |
| Tauri 2 | El README indica doble MIT y Apache-2.0 | Auditar crates transitivos |
| ONLYOFFICE | AGPL-3.0, no se usa su código | — |

**Observación para la privacidad.** Según el manifiesto de `gpui` en la rama principal (2026-10-22), `gpui` depende del crate `http_client`. El plan debe comprobar en el commit fijado que la aplicación no abre conexiones de red (Constitución, principio I).

**Fuentes.**
- IronCalc: https://github.com/ironcalc/IronCalc · https://crates.io/api/v1/crates/ironcalc · https://raw.githubusercontent.com/ironcalc/IronCalc/main/LICENSE-MIT · https://raw.githubusercontent.com/ironcalc/IronCalc/main/LICENSE-Apache-2.0
- `calamine`: https://crates.io/api/v1/crates/calamine · https://github.com/tafia/calamine
- `rust_xlsxwriter`: https://docs.rs/rust_xlsxwriter/latest/rust_xlsxwriter/struct.Formula.html
- Zed (README, licencias): https://github.com/zed-industries/zed
- `gpui`: https://raw.githubusercontent.com/zed-industries/zed/main/crates/gpui/Cargo.toml
- Tauri: https://github.com/tauri-apps/tauri
- ONLYOFFICE: repositorios de D-04.

---

## D-16 — Arquitectura de la suite; IronCalc como pieza y no como «el motor»

**Decisión.** Dokumentz se construye como **una suite** (hoja, documentos, presentaciones) con una **base común** compartida y un **motor por tipo de documento**, no como una PoC suelta ni como «un solo motor». IronCalc es la calculadora de la hoja (vive dentro de `dokumentz-sheet`); no es el motor de la suite. La PoC 001 construye **piezas de producto** (`dokumentz-sheet`, `dokumentz-metrics`, `dokumentz-script` y las herramientas), no un instrumento desechable; solo la interfaz perdedora se descarta. La arquitectura completa, las capas, las fases y la hoja de ruta están en [`../000-arquitectura-suite/arquitectura.md`](../000-arquitectura-suite/arquitectura.md); aquí solo se registra la razón y los hechos. `[NECESITA ACLARACIÓN: confirmar aplicación única con pestañas frente a tres aplicaciones separadas; propuesta por defecto: aplicación única (000).]`

**Por qué.**
- **Ninguna suite real es «un solo motor».** Microsoft Office tiene motores distintos para Excel, Word y PowerPoint sobre una base común compartida; LibreOffice tiene Calc, Writer e Impress sobre módulos comunes. En el ONLYOFFICE instalado en el Mac del usuario (D-12, inspección local), `sdkjs/common` pesa 37 MB, más que `word` (10 MB) y `cell` (11 MB) juntos: lo compartido es lo más grande.
- **No existe un motor completo, rápido, fiel, local y permisivo.** Los únicos motores completos existentes son LibreOffice (fidelidad y aspecto que no convencieron; C++ enorme; MPL-2.0 más LGPL; LibreOfficeKit entrega teselas con API experimental, D-05) y ONLYOFFICE (JavaScript, lento; AGPL-3.0, D-04). Ese es el hueco que llena Dokumentz.
- **IronCalc solo cubre la hoja.** Versión 0.9.1, MIT OR Apache-2.0 (D-15); su tabla de idiomas lista 497 nombres de funciones (Excel tiene alrededor de 500); **no se sabe cuántas calculan igual que Excel** (se mide en SC-007, D-17); reevalúa todo el libro en cada edición (D-02).
- **Riesgos de la versión anterior de la PoC que esto corrige:** (a) la PoC era un instrumento desechable; (b) IronCalc se veía como «el motor» y es solo la calculadora; (c) lo más difícil de la suite (paginar documentos con fidelidad a Word) quedaba para el final, por lo que se adelanta la PoC 002 de documentos (000, fase 0b); (d) la PoC medía velocidad e interfaz pero no el motivo original del proyecto: que las alternativas **no abren bien los archivos de la gente** (además: LibreOffice se siente viejo y descuadra; Google lo logra pero en la nube; hay que cubrir el día a día, incluidas las macros; debe sentirse como Office, como ONLYOFFICE, pero rápido). Lo último se mide ahora con SC-007.
- **Licencia:** el proyecto es MIT OR Apache-2.0 y se prohíben las dependencias GPL, LGPL y AGPL (Constitución v1.1.0), lo que descarta incrustar LibreOffice o ONLYOFFICE como motor de la suite.

**Alternativas consideradas.**
- *PoC como instrumento desechable y rehacer todo en la fase 1:* descartada, tira trabajo y aplaza lo difícil.
- *Tres aplicaciones totalmente separadas sin base común:* descartada, duplica el motor de texto, el dibujo y el paquete de Office, que es justo lo que las suites reales comparten.
- *LibreOfficeKit como motor de documentos:* no se descarta; se compara en la PoC 002 (D-05 sigue cerrada para la PoC 001).

**Fuentes.**
- Arquitectura y hoja de ruta: `../000-arquitectura-suite/arquitectura.md`.
- Inspección local de ONLYOFFICE 10.0.0 (D-12); LibreOfficeKit: https://docs.libreoffice.org/libreofficekit.html
- IronCalc: https://crates.io/api/v1/crates/ironcalc · https://github.com/ironcalc/IronCalc · evaluación completa en cada edición: https://github.com/ironcalc/IronCalc/blob/v0.9.1/base/src/evaluation.rs
- La estructura interna de Office y de LibreOffice (motores por aplicación sobre una base común) es un hecho aportado por el dueño; no tiene fuente primaria citada aquí (ver «Hechos sin verificar»).

---

## D-17 — Método de compatibilidad con los valores que Excel guardó en el archivo

**Decisión.** La compatibilidad del motor (US5, FR-014, M-09, SC-007) se mide **sin necesitar Excel**: los `.xlsx` guardan, junto a cada fórmula, el último resultado que calculó el programa que los produjo. `tools/compat-check` abre cada archivo de A4 con el motor de la hoja, lo recalcula y compara cada celda con fórmula contra el valor guardado, **solo en archivos producidos por Excel**. El lector de los valores guardados y de las fórmulas es **independiente del motor**: `calamine` 0.37.0 (MIT). El detalle del procedimiento está en `plan.md` §9.

**Por qué.**
- **Lector independiente.** Si el mismo motor leyera el valor esperado, se compararía consigo mismo. `calamine` es un lector puro de Rust (xls, xlsx, xlsm, xlsb, ods), licencia MIT, versión 0.37.0 publicada el 2026-10-18, sin ejecutar macros.
- **Verificado: `calamine` 0.37.0 devuelve el valor guardado de las celdas con fórmula.** (1) La documentación de `XlsxCellFormula` describe el campo `value` como «Literal or cached value associated with the cell» y el campo `formula` como el texto de la fórmula; `XlsxCellReader::next_cell_with_formula` devuelve «cached or literal value» más la fórmula expandida (https://docs.rs/calamine/0.37.0/calamine/struct.XlsxCellFormula.html y https://docs.rs/calamine/0.37.0/calamine/struct.XlsxCellReader.html). (2) Prueba local el 2026-10-22 con `calamine` 0.37.0 y `rust_xlsxwriter` 0.101.0: un libro con A1=2, B1=3 y `=A1+B1` con resultado guardado forzado a 7 devolvió `Float(7.0)` en `worksheet_range` (no 5, que es lo que calcularía un motor) y `A1+B1` (sin el signo igual) en `worksheet_formula`; igual con `Worksheet::set_formula_result`. Es decir, `worksheet_range` entrega el valor guardado y `worksheet_formula` entrega el texto de la fórmula.
- **Detección del productor.** El campo `Application` de `docProps/app.xml` nombra la aplicación que creó el archivo (es la primera de las propiedades extendidas de ECMA-376). Un libro guardado por Excel trae «Microsoft Excel» (https://forum.aspose.com/t/application-xml-node-in-app-xml-is-empty-for-new-workbooks/227999). Los archivos de LibreOffice, Apache POI u otro productor se **informan aparte**: su valor guardado no es el de Excel. El valor exacto que escriben POI y LibreOffice **no se verificó** aquí; se confirma con los archivos de A4.
- **Funciones volátiles.** Las funciones cuyo resultado depende del reloj o del azar no pueden coincidir con el valor guardado (el archivo se guardó en otro momento): `HOY`/`TODAY`, `AHORA`/`NOW`, `ALEATORIO`/`RAND`, `ALEATORIO.ENTRE`/`RANDBETWEEN` y `RANDARRAY`. `CELL` e `INFO` son volátiles según sus argumentos y dependen del entorno. Se excluyen de (b) y se cuentan aparte; `DESREF`/`OFFSET` e `INDIRECTO`/`INDIRECT` también son volátiles para el recálculo pero son **deterministas**, así que sí se comparan. Lista de funciones volátiles: https://exceljet.net/glossary/volatile-function (fuente secundaria; no se encontró la página equivalente de Microsoft).
- **Tolerancias (decididas).** Números con tolerancia relativa 1e-9; texto y booleanos exactos; errores del mismo tipo (`#DIV/0!`, `#N/A`, `#NAME?`, `#REF!`, `#VALUE!`…).
- **Seguridad.** `calamine` solo lee; la herramienta nunca ejecuta macros ni abre los archivos en Excel. Los archivos de internet son no confiables (spec, aclaración 12).
- **Fixtures de la herramienta.** `.xlsx` pequeños generados con `rust_xlsxwriter` (MIT OR Apache-2.0) usando `Formula::set_result` o `Worksheet::set_formula_result` para fijar el valor guardado, de modo que haya coincidencias y discrepancias conocidas (autoprueba).

**Limitación conocida (verificada) del campo `Application`.** Las bibliotecas que generan `.xlsx` copian la cadena «Microsoft Excel»: `rust_xlsxwriter` 0.101.0 escribe `<Application>Microsoft Excel</Application>`, `AppVersion` 12.0000 y además `fullCalcOnLoad="1"` en `xl/workbook.xml` (comprobado en un archivo generado localmente el 2026-10-22), y XlsxWriter (Python) escribe la misma cadena (https://raw.githubusercontent.com/jmcnamara/XlsxWriter/main/xlsxwriter/app.py). Un archivo así parecería «producido por Excel» aunque sus valores guardados no sean de Excel (por defecto guardan 0 hasta que se recalcula). **Resuelto por defecto (2026-10-22):** se añade una segunda señal. Un archivo con `Application` «Microsoft Excel» se clasifica `generador_sospechoso` si trae `fullCalcOnLoad="1"` en `calcPr` o si ninguna celda con fórmula trae valor guardado; esos archivos no entran en M-09(b) y se informan aparte (`plan.md` §9.10, paso 1). `AppVersion` no se usa como señal porque varía entre versiones de Excel.

**Alternativas consideradas.**
- *Comparar contra Excel real ejecutándolo (automatización):* descartada como método principal; Excel es opcional (spec), la automatización es frágil y abrir archivos de internet en Excel con macros es un riesgo. Si Excel está disponible puede servir de verificación puntual.
- *Comparar contra LibreOffice (recalcular con LibreOffice como juez):* es un juez de compatibilidad (D-05), pero sus resultados no son los de Excel; no sustituye al valor guardado por Excel.
- *Leer los valores esperados con el propio IronCalc:* descartada, no es independiente.
- *Otros lectores de `.xlsx` en Rust:* no se evaluaron; `rust_xlsxwriter` solo escribe.

**Fuentes.**
- `calamine`: https://crates.io/api/v1/crates/calamine · https://docs.rs/calamine/0.37.0/calamine/struct.XlsxCellFormula.html · https://docs.rs/calamine/0.37.0/calamine/struct.XlsxCellReader.html · https://docs.rs/calamine/0.37.0/calamine/enum.Data.html · https://github.com/tafia/calamine
- Código de lectura de `<f>` y `<v>`: https://raw.githubusercontent.com/tafia/calamine/master/src/xlsx/cells_reader.rs
- `rust_xlsxwriter` (`Formula::set_result`, `Worksheet::set_formula_result`): https://docs.rs/rust_xlsxwriter/latest/rust_xlsxwriter/struct.Formula.html · https://docs.rs/rust_xlsxwriter/0.101.0/rust_xlsxwriter/worksheet/struct.Worksheet.html
- Campo `Application` de `docProps/app.xml`: https://forum.aspose.com/t/application-xml-node-in-app-xml-is-empty-for-new-workbooks/227999 · https://raw.githubusercontent.com/jmcnamara/XlsxWriter/main/xlsxwriter/app.py · https://www.libreoffice.org/bugzilla/show_bug.cgi?id=89775 (lista las propiedades extendidas de ECMA-376; la primera es `Application`). La definición normativa está en ECMA-376 parte 1; no se consultó directamente.
- Funciones volátiles: https://exceljet.net/glossary/volatile-function
- Corpus de A4: D-15.

---

## D-18 — Nombres de producto en lugar de `poc-*`

**Decisión.** Como la PoC construye piezas de producto (D-16), los crates, herramientas y esquemas llevan nombres de producto y no el prefijo `poc-`. Los nombres antiguos no deben reaparecer en la documentación ni en el código.

| Antes | Ahora | Qué es |
|---|---|---|
| `poc-core` (`crates/poc-core`) | `dokumentz-sheet` (`crates/dokumentz-sheet`) | Motor de la hoja: IronCalc, vista de cuadrícula y comandos |
| `poc-metrics` | `dokumentz-metrics` | Instrumentación (permanente y compartida) |
| `poc-script`, `poc-script-check` | `dokumentz-script`, `dokumentz-script-check` | Guiones de prueba compartidos |
| `poc-gen` (`crates/poc-gen`) | `testdata-gen` (`tools/testdata-gen`) | Generador de A1–A3 (herramienta) |
| `poc-tauri`, `poc-gpui` | `dokumentz-app-tauri`, `dokumentz-app-gpui` | Las dos interfaces candidatas (`apps/tauri`, `apps/gpui`) |
| `POC_METRICS_OUT` | `DOKUMENTZ_METRICS_OUT` | Variable de entorno de la instrumentación |
| esquema `poc-results/1` | `dokumentz-bench/1` | Esquema JSON de resultados |
| (nuevo) | `tools/compat-check` | Herramienta de compatibilidad (D-17) |

**Por qué.** Evita renombrar todo al pasar a la fase 1 y deja claro qué se conserva (los crates `dokumentz-*` y las herramientas) y qué es provisional (solo la interfaz perdedora de `apps/`).

**Alternativas consideradas.** *Mantener `poc-*` y renombrar en la fase 1:* descartada, cuesta un cambio masivo y confunde el historial.

**Fuentes.** Decisión del dueño (2026-10-22); estructura completa en `plan.md` §4 y en `../000-arquitectura-suite/arquitectura.md`.

---

## Hechos sin verificar o pendientes

| Asunto | Estado |
|---|---|
| Patrón del textarea oculto en Google Docs y ONLYOFFICE | Sin fuente primaria (D-07); no condiciona la decisión |
| Disponibilidad de EditContext en WebView2 | No verificada (D-07) |
| Términos vigentes en 2026 de la licencia de la cinta; sentencia final y apelaciones de *Microsoft v. Corel* | No verificados (D-14) |
| Costo real del recálculo completo de IronCalc en cada edición | El comportamiento (reevalúa todo el libro) está verificado en el código de v0.9.1; su costo se mide en M-05 (D-02) |
| Cuántas de las 497 funciones de IronCalc calculan igual que Excel | Se mide en SC-007 / M-09(d) (D-16, D-17) |
| Valor de `Application` que escriben Apache POI y LibreOffice, y si todo archivo de A4 trae `docProps/app.xml` | Sin verificar; se confirma al preparar A4. Sin `app.xml` el archivo se informa aparte (D-17) |
| Archivos generados por bibliotecas que escriben «Microsoft Excel» en `Application` | Limitación verificada con `rust_xlsxwriter` 0.101.0; falta decidir si se añade una segunda señal (D-17) |
| Estructura interna de Office y de LibreOffice (motores por aplicación sobre una base común) | Hecho aportado por el dueño; sin fuente primaria citada (D-16) |
| Lista de funciones volátiles de Excel según Microsoft | Fuente secundaria (Exceljet); no se encontró la página de Microsoft (D-17) |
| Licencias de los crates de Zed de los que depende `gpui` | Directas: todas Apache-2.0 (verificado 2026-10-22). Transitivas: auditoría con `cargo deny` en el commit fijado (D-15) |
| Calidad del IME de GPUI en Windows y Linux | Se prueba en SC-004 (D-07) |
| Otros motores de hojas en Rust y otros marcos nativos (iced, etc.) | No evaluados (D-02, D-03) |
| ONLYOFFICE en Windows y Linux | No inspeccionado (D-12) |

## Lista de fuentes

**Motor y formato**
- https://github.com/ironcalc/IronCalc
- https://docs.rs/ironcalc_base/latest/ironcalc_base/struct.UserModel.html
- https://docs.rs/ironcalc/latest/ironcalc/import/index.html

**Compatibilidad con archivos reales (SC-007)**
- https://crates.io/api/v1/crates/ironcalc
- https://crates.io/api/v1/crates/calamine
- https://docs.rs/calamine/0.37.0/calamine/struct.XlsxCellFormula.html
- https://docs.rs/calamine/0.37.0/calamine/struct.XlsxCellReader.html
- https://raw.githubusercontent.com/tafia/calamine/master/src/xlsx/cells_reader.rs
- https://docs.rs/rust_xlsxwriter/latest/rust_xlsxwriter/struct.Formula.html
- https://docs.rs/rust_xlsxwriter/0.101.0/rust_xlsxwriter/worksheet/struct.Worksheet.html
- https://raw.githubusercontent.com/ironcalc/IronCalc/main/LICENSE-MIT
- https://raw.githubusercontent.com/ironcalc/IronCalc/main/LICENSE-Apache-2.0
- https://github.com/ironcalc/IronCalc/blob/v0.9.1/base/src/evaluation.rs
- https://github.com/ironcalc/IronCalc/blob/v0.9.1/base/src/user_model/common.rs
- https://raw.githubusercontent.com/jmcnamara/XlsxWriter/main/xlsxwriter/app.py
- https://forum.aspose.com/t/application-xml-node-in-app-xml-is-empty-for-new-workbooks/227999
- https://www.libreoffice.org/bugzilla/show_bug.cgi?id=89775
- https://exceljet.net/glossary/volatile-function

**Arquitectura de la suite**
- `../000-arquitectura-suite/arquitectura.md`

**Interfaces**
- https://github.com/tauri-apps/tauri/releases
- https://github.com/tauri-apps/tauri
- https://v2.tauri.app/reference/webview-versions/
- https://v2.tauri.app/develop/debug/linux-graphics/
- https://v2.tauri.app/develop/calling-rust/
- https://learn.microsoft.com/en-IE/microsoft-edge/webview2/concepts/developer-guide
- https://caniuse.com/mdn-api_editcontext
- https://github.com/zed-industries/zed
- https://raw.githubusercontent.com/zed-industries/zed/main/crates/gpui/Cargo.toml
- https://docs.rs/gpui/latest/gpui/trait.EntityInputHandler.html
- https://zed.dev/blog/zed-1-0
- https://zed.dev/blog/videogame

**Rendimiento en hojas web**
- https://www.quadratichq.com/blog/building-a-high-performance-spreadsheet-renderer-why-we-chose-webgl-over-html
- https://deephaven.io/blog/2022/01/24/displaying-a-quadrillion-rows/
- https://web.dev/case-studies/google-sheets-wasmgc

**Testimonios y precedentes**
- https://opensource.porsche.com/porschedigital-technology-radar/tools/tauri/
- https://gethopp.app/blog/hate-webkit
- https://www.collaboraonline.com/blog/collabora-online-now-available-on-desktop/
- https://docs.libreoffice.org/libreofficekit.html

**ONLYOFFICE y licencias**
- https://github.com/ONLYOFFICE/sdkjs · https://github.com/ONLYOFFICE/web-apps · https://github.com/ONLYOFFICE/core
- https://forum.onlyoffice.com/t/fast-but-still-not-fast-enough/10209
- https://linuxiac.com/fsf-says-onlyoffice-cannot-use-agpl-to-restrict-forks/

**Riesgo legal de interfaz**
- https://visualstudiomagazine.com/blogs/desmond-file/2007/01/microsoft-gets-fluent.aspx
- https://www.itwriting.com/blog/885-developers-will-you-or-have-you-signed-for-ribbon-rights.html
- https://ipwatchdog.com/2018/02/13/93710/
- https://patentlawcenter.pli.edu/?p=10843
- https://office-watch.com/2018/microsoft-wins-office-patent-battle-corel/
