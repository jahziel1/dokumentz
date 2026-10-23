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
| D-15 | Licencias de las dependencias principales | Verificación pendiente en el plan |

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
- Licencia permisiva (GitHub indica Apache-2.0; el README indica doble MIT/Apache-2.0). Financiado por NLnet.

**Riesgos que la PoC mide o vigila.**
- Es **pre-1.0**: la API puede cambiar; por eso se fija la versión.
- La documentación de `pause_evaluation`/`resume_evaluation` y la evaluación automática no dice si el recálculo es **incremental**: se mide en M-05.
- La fidelidad de ida y vuelta del `.xlsx` se mide con SC-005.
- La discrepancia de licencia entre GitHub y el README se resuelve revisando los archivos `LICENSE` del repositorio antes de depender de él (D-15).

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
| IronCalc 0.9.1 | GitHub indica Apache-2.0; el README indica doble MIT/Apache-2.0 | Revisar los archivos `LICENSE` y las licencias de sus dependencias |
| GPUI | Apache-2.0 (`license = "Apache-2.0"` en `crates/gpui/Cargo.toml`) | El repositorio de Zed es, en su conjunto, principalmente GPL-3.0-or-later «con componentes Apache-2.0 donde se indica». **Verificado el 2026-10-22 en la rama principal:** todas las dependencias internas directas de `gpui` dentro del espacio de trabajo de Zed son Apache-2.0 (`bench_metrics`, `collections`, `gpui_macros`, `gpui_platform`, `gpui_shared_string`, `gpui_util`, `http_client`, `path`, `refineable`, `reqwest_client`, `scheduler`, `sum_tree`, `util_macros`). Falta repetir la auditoría **transitiva** en el commit fijado (con `cargo deny check licenses` en CI) |
| Tauri 2 | El README indica doble MIT y Apache-2.0 | Auditar crates transitivos |
| ONLYOFFICE | AGPL-3.0, no se usa su código | — |

**Observación para la privacidad.** Según el manifiesto de `gpui` en la rama principal (2026-10-22), `gpui` depende del crate `http_client`. El plan debe comprobar en el commit fijado que la aplicación no abre conexiones de red (Constitución, principio I).

**Fuentes.**
- IronCalc: https://github.com/ironcalc/IronCalc
- Zed (README, licencias): https://github.com/zed-industries/zed
- `gpui`: https://raw.githubusercontent.com/zed-industries/zed/main/crates/gpui/Cargo.toml
- Tauri: https://github.com/tauri-apps/tauri
- ONLYOFFICE: repositorios de D-04.

---

## Hechos sin verificar o pendientes

| Asunto | Estado |
|---|---|
| Patrón del textarea oculto en Google Docs y ONLYOFFICE | Sin fuente primaria (D-07); no condiciona la decisión |
| Disponibilidad de EditContext en WebView2 | No verificada (D-07) |
| Términos vigentes en 2026 de la licencia de la cinta; sentencia final y apelaciones de *Microsoft v. Corel* | No verificados (D-14) |
| Recálculo incremental de IronCalc | Se mide en M-05 (D-02) |
| Licencias de los crates de Zed de los que depende `gpui` | Directas: todas Apache-2.0 (verificado 2026-10-22). Transitivas: auditoría con `cargo deny` en el commit fijado (D-15) |
| Calidad del IME de GPUI en Windows y Linux | Se prueba en SC-004 (D-07) |
| Otros motores de hojas en Rust y otros marcos nativos (iced, etc.) | No evaluados (D-02, D-03) |
| ONLYOFFICE en Windows y Linux | No inspeccionado (D-12) |

## Lista de fuentes

**Motor y formato**
- https://github.com/ironcalc/IronCalc
- https://docs.rs/ironcalc_base/latest/ironcalc_base/struct.UserModel.html
- https://docs.rs/ironcalc/latest/ironcalc/import/index.html

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
