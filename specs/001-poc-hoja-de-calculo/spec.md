# Especificación 001 — PoC de hoja de cálculo

**Rama sugerida:** `001-poc-hoja-de-calculo`
**Fecha:** 2026-10-22
**Estado:** Borrador
**Documentos relacionados:** [Constitución](../constitucion.md) · [research.md](research.md) (decisiones y fuentes) · `plan.md` (el CÓMO) · `tasks.md` (tareas)

> Este documento define el **QUÉ** y el **POR QUÉ**. No nombra tecnologías salvo en la sección 9 («Regla de decisión Tauri vs GPUI» y regla del motor) y en «Supuestos, restricciones y dependencias». En el resto se habla de la **interfaz A** y la **interfaz B** (definidas en esa última sección). Las fuentes de los hechos citados están en `research.md`.

## 1. Contexto

Se quiere una suite de oficina libre, gratuita y local (hoja de cálculo, documentos y presentaciones) que sea una alternativa real a Microsoft Office, **más rápida** que ONLYOFFICE (que gusta visualmente pero se siente lenta; ver Constitución) y que **abra bien los archivos reales de la gente**, que es el motivo original del proyecto. Esta es la **fase 0a** de la hoja de ruta de la suite, definida en [`../000-arquitectura-suite/arquitectura.md`](../000-arquitectura-suite/arquitectura.md): 0a PoC 001 hoja → 0b PoC 002 documentos → 1 base común + hoja básica → 2 documentos básicos → 3 presentaciones básicas → 4 día a día en las tres aplicaciones → 5 macros.

La PoC construye una hoja de cálculo mínima en **dos interfaces candidatas** que ofrecen exactamente lo mismo sobre un único núcleo, las mide con el mismo método que a ONLYOFFICE 10.0.0 y decide con datos cuál queda. Además mide si el **motor de la hoja** abre los archivos reales con los mismos resultados que Excel (SC-007). **La PoC construye piezas de producto, no un instrumento desechable:** el motor de la hoja, la instrumentación, los guiones de prueba y las herramientas de generación, medición y compatibilidad se conservan en las fases siguientes; solo la interfaz perdedora se descarta.

Alcance de interfaz: **cuadrícula + barra de fórmulas + pestañas de hojas**. Sin cinta (la cinta es fase 1).

## 2. Objetivo y preguntas que debe responder

El objetivo es responder con **datos medidos** (no con opiniones) las siguientes preguntas:

| # | Pregunta | Se responde con |
|---|---|---|
| 1 | ¿Un núcleo especializado con una interfaz delgada hace que una hoja grande se sienta **claramente más rápida** que en ONLYOFFICE? | M-01…M-08 frente a la línea base de ONLYOFFICE; SC-001, SC-002, SC-003 |
| 2 | ¿Qué interfaz debe quedar como definitiva: la A o la B (definidas en la sección 11)? | La regla de decisión Tauri vs GPUI (sección 9) aplicada a las medidas |
| 3 | ¿Funciona la **regla de oro**: la interfaz solo dibuja y recibe entradas, y todo lo demás vive en el núcleo? | Ambas interfaces ofrecen los mismos FR-001…FR-011 sin lógica de documento propia; revisión de diff (Constitución, principio III) |
| 4 | ¿Se comporta bien en **macOS, Windows y Linux** (incluido Linux con NVIDIA)? | Medidas y pruebas en hardware real de cada plataforma; SC-002, SC-004, SC-006 |
| 5 | ¿El motor de la hoja **abre los archivos reales de la gente con los mismos resultados que Excel**? | M-09 sobre los archivos reales A4; SC-007 (decide sobre el motor, no sobre la interfaz) |

## 3. Escenarios de usuario y pruebas

Prioridades: **P1** = sin esto la PoC no responde sus preguntas; **P2** = necesario para uso real pero no decide la pregunta principal.

### US1 — Abrir y recorrer una hoja grande (P1)

*Como persona que trabaja con hojas grandes, quiero abrir un archivo `.xlsx` de 100 000+ filas y recorrerlo con teclado, ratón y trackpad sin esperas ni tirones.*

**Requisitos:** FR-001, FR-002, FR-003, FR-004 · **Medidas:** M-01, M-02, M-03

**Por qué esta prioridad.** Es la promesa central de la PoC (pregunta 1) y la base de todo lo demás: sin abrir y recorrer una hoja grande no hay nada que editar ni comparar.

**Prueba independiente.** Con el archivo A1: abrirlo por argumento y por diálogo, desplazarse 10 s de forma continua y navegar con teclado, registrando M-01, M-02 y M-03. No requiere edición, fórmulas ni guardado.

**Escenarios de aceptación**

1. **Dado** que A1 existe en disco, **cuando** inicio la aplicación pasando la ruta de A1 como argumento de línea de comandos, **entonces** la ventana muestra la primera pantalla con datos sin intervención del usuario y se registra M-02.
2. **Dado** la aplicación abierta sin archivo, **cuando** elijo A1 en el diálogo de abrir, **entonces** obtengo el mismo resultado que en el escenario 1.
3. **Dado** A1 abierto, **cuando** miro las celdas visibles, **entonces** veo cabeceras de filas y de columnas, los números con su formato (por ejemplo, fechas como fechas y no como números de serie) y los estilos existentes en el archivo (negrita, cursiva, color de texto, color de relleno, alineación) con el mismo aspecto que en ONLYOFFICE.
4. **Dado** A1 abierto, **cuando** me desplazo de forma continua durante 10 s con la rueda, el trackpad y arrastrando la barra, **entonces** el contenido sigue al gesto sin saltos, no quedan zonas en blanco duraderas y se registran los resultados de M-03.
5. **Dado** A1 abierto, **cuando** arrastro la barra de desplazamiento hasta el final, **entonces** se muestra la última fila de la hoja y las cabeceras de fila coinciden con los datos (sin desfase).
6. **Dado** una celda seleccionada de A1, **cuando** uso flechas, Ctrl/Cmd+flechas (ir al borde del bloque de datos), Mayús+flechas (extender el rango), RePág/AvPág (mover una pantalla), clic y arrastre, **entonces** la selección se comporta como en Office y la celda activa permanece visible.
7. **Dado** A1 abierto, **cuando** me desplazo, **entonces** el número de celdas que se dibujan por cuadro depende del área visible y no del tamaño de la hoja (FR-003; se verifica con la instrumentación, FR-012).

### US2 — Editar y recalcular, en español (P1)

*Como persona hispanohablante, quiero escribir y editar celdas con fórmulas en español, con mi teclado y mi método de entrada, y ver el recálculo al instante.*

**Requisitos:** FR-005, FR-006, FR-007, FR-008 · **Medidas:** M-04, M-05

**Por qué esta prioridad.** Es lo que convierte la cuadrícula en una hoja de cálculo. M-04 y M-05 son parte de la promesa de velocidad. La entrada de texto internacional (IME) es el mayor riesgo técnico de ambas interfaces y conviene descubrirlo pronto.

**Prueba independiente.** En un libro nuevo o en A2: escribir valores y fórmulas, editar, deshacer y rehacer, y medir M-04 y M-05. No requiere guardar ni copiar y pegar.

**Escenarios de aceptación**

1. **Dado** una celda seleccionada, **cuando** escribo texto directamente y pulso Intro, **entonces** la celda guarda el valor y la selección baja a la celda siguiente; con Tab, la selección pasa a la celda de la derecha (como en Office).
2. **Dado** una celda con contenido, **cuando** pulso F2 (Windows/Linux) o el atajo equivalente de macOS, o hago clic en la barra de fórmulas, **entonces** entro en edición con el contenido actual; **cuando** pulso Esc, **entonces** se descartan los cambios y la celda conserva su valor anterior.
3. **Dado** una celda en edición, **cuando** escribo en la celda o en la barra de fórmulas, **entonces** ambas muestran el mismo texto mientras escribo.
4. **Dado** idioma de fórmulas español, **cuando** escribo `=SUMA(A1:A3)`, `=BUSCARV(...)`, `=BUSCARX(...)` o `=SI.ERROR(...)`, **entonces** la fórmula se acepta y se calcula; con las funciones equivalentes en inglés (`=SUM`, `=VLOOKUP`, `=XLOOKUP`, `=IFERROR`) ocurre lo mismo.
5. **Dado** A2 y el idioma español, **cuando** abro el archivo, **entonces** las fórmulas se muestran con nombres de función en español y se calculan sin errores.
6. **Dado** A2 abierto, **cuando** cambio la celda de la que dependen ≥10 000 fórmulas y confirmo, **entonces** los valores visibles se actualizan, la aplicación vuelve a aceptar entrada y se registra M-05.
7. **Dado** una celda en edición con un teclado en español, **cuando** escribo con teclas muertas (por ejemplo `´` + `a`, `¨` + `u`) y con `ñ`, **entonces** aparecen á, ü y ñ correctamente.
8. **Dado** un sistema con un método de entrada japonés o chino (IME), **cuando** compongo texto en una celda, **entonces** la composición se ve en la celda, los candidatos aparecen junto a ella, al confirmar se inserta el texto elegido y Esc durante la composición cancela solo la composición.
9. **Dado** macOS y una celda en edición, **cuando** uso el dictado del sistema, **entonces** el texto dictado aparece en la celda.
10. **Dado** una edición confirmada, **cuando** deshago con el atajo del sistema (Cmd/Ctrl+Z), **entonces** la celda y las fórmulas dependientes vuelven a su valor anterior; **cuando** rehago con el atajo del sistema, **entonces** se reaplica el cambio.
11. **Dado** que se registra M-04 en A1 o A2, **cuando** escribo en una celda, **entonces** cada carácter es visible en la celda con una latencia que se mide desde la pulsación de la tecla.

### US3 — Varias hojas, copiar/pegar y guardar sin pérdidas (P2)

*Como persona que intercambia archivos con otras aplicaciones, quiero cambiar de hoja, copiar y pegar con otras aplicaciones y guardar un `.xlsx` que se abra sin problemas.*

**Requisitos:** FR-009, FR-010, FR-011 · **Medida:** M-07

**Por qué esta prioridad.** Es necesaria para usar la hoja de verdad y para demostrar la compatibilidad (Constitución, principio IV), pero no decide ni la velocidad ni la interfaz: la regla de decisión se apoya en SC-001…SC-004. Se hace después de las historias P1.

**Prueba independiente.** Con A3 (y A1/A2 para integridad): cambiar de hoja, copiar y pegar, guardar y reabrir el resultado en ONLYOFFICE. No requiere medir rendimiento de desplazamiento ni de tecleo.

**Escenarios de aceptación**

1. **Dado** A3 abierto (5 hojas), **cuando** hago clic en cada pestaña de hoja de la parte inferior, **entonces** se muestra el contenido de esa hoja y la pestaña activa queda marcada.
2. **Dado** un rango seleccionado, **cuando** lo copio y lo pego en otra zona de la misma hoja o de otra hoja, **entonces** los valores se copian conforme al comportamiento definido en las aclaraciones.
3. **Dado** un rango copiado en la aplicación, **cuando** lo pego en ONLYOFFICE o en Excel, **entonces** llega como texto tabulado (columnas separadas por tabulador, filas por salto de línea).
4. **Dado** un rango copiado desde ONLYOFFICE o Excel, **cuando** lo pego en la aplicación sobre la celda activa, **entonces** se rellenan las celdas correspondientes hacia abajo y hacia la derecha.
5. **Dado** A1, A2 o A3 abierto (con o sin ediciones), **cuando** uso «Guardar como» `.xlsx` y reabro el resultado en ONLYOFFICE (y en Excel si está disponible), **entonces** se abre sin errores ni avisos de reparación y con los mismos valores y fórmulas; se registra M-07.
6. **Dado** el archivo guardado, **cuando** lo reabro en la propia aplicación, **entonces** el contenido es el mismo que antes de guardar.
7. **Dado** A1, A2 y A3, **cuando** se ejecuta la comparación automática de ida y vuelta (abrir → guardar → reabrir), **entonces** no hay diferencias de valores ni de fórmulas (SC-005).

### US4 — Medir y comparar contra ONLYOFFICE en las tres plataformas, y decidir la interfaz (P1)

*Como responsable del proyecto, quiero un informe comparable de ambas interfaces y de ONLYOFFICE en macOS, Windows y Linux, para decidir con datos.*

**Requisitos:** FR-012, FR-013 · **Medidas:** M-06, M-08 (y consolidación de M-01…M-07) · **Criterios:** SC-001…SC-006 y la regla de decisión

**Por qué esta prioridad.** Es la razón de ser de la PoC. Sin archivos de prueba reproducibles ni mediciones comparables no se puede responder ninguna de las cuatro preguntas. Generador e instrumentación se necesitan desde el principio para que las demás historias se puedan medir.

**Prueba independiente.** Generar A1–A3 con semilla fija, ejecutar una corrida de medición en una plataforma con las tres aplicaciones y producir el informe con los datos disponibles hasta ese momento.

**Escenarios de aceptación**

1. **Dado** una semilla fija, **cuando** ejecuto el generador dos veces, **entonces** A1, A2 y A3 tienen el mismo contenido celda a celda y cumplen la forma definida en la sección 7.
2. **Dado** que activo la bandera de medidas, **cuando** uso la aplicación, **entonces** escribe un JSON de resultados en el equipo y no abre conexiones de red; **cuando** no la activo, **entonces** no escribe ni mide nada.
3. **Dado** las tres aplicaciones (interfaz A, interfaz B y ONLYOFFICE 10.0.0) en la misma máquina, **cuando** se aplica el mismo método de medición externo a todas, **entonces** se obtienen para M-01…M-08, con 5 repeticiones, la mediana y el p95, en frío y en caliente donde aplique, y además las dos interfaces se miden con su instrumentación interna.
4. **Dado** los resultados de cada plataforma, **cuando** se evalúan SC-001…SC-006, **entonces** cada criterio queda como cumple / no cumple con el dato que lo sustenta; lo no medido se marca «sin datos» y nunca se da por cumplido.
5. **Dado** la evaluación, **cuando** se aplica la regla de decisión (sección 9), **entonces** el informe indica cuál de sus salidas corresponde y con qué datos.
6. **Dado** Linux con NVIDIA (si hay hardware), **cuando** se mide, **entonces** el informe registra la configuración y las mitigaciones aplicadas, de modo que no se confundan con el comportamiento por defecto.

### US5 — Abrir los archivos reales de la gente con los mismos resultados que Excel (P1)

*Como persona que ya tiene sus libros hechos en Excel, quiero abrirlos en la hoja y ver los mismos valores que veía en Excel, para poder dejar de depender de Office.*

**Requisitos:** FR-014 · **Medida:** M-09 · **Criterio:** SC-007 y la regla del motor (sección 9)

**Por qué esta prioridad.** Es el motivo original del proyecto: las alternativas existentes no abren bien los archivos de la gente (descuadran formatos y resultados). La PoC medía velocidad e interfaz, pero no si el **motor** de la hoja calcula lo mismo que Excel; si no lo hace, ninguna interfaz rápida sirve. La pregunta 5 se responde con esta historia y decide sobre el motor, no sobre la interfaz.

**Prueba independiente.** Con A4 (archivos reales) y con archivos pequeños construidos a propósito con valor guardado conocido: ejecutar la herramienta de compatibilidad (FR-014) sobre cada archivo y producir el informe M-09 por archivo y por fuente. No requiere ninguna de las dos interfaces, ni edición, ni guardado.

**Escenarios de aceptación**

1. **Dado** un archivo de A4, **cuando** la herramienta lo abre con el motor de la hoja, **entonces** registra si abrió sin error (y qué error, si lo hubo) y si hubo un cierre inesperado; un archivo que falla no detiene la corrida de los demás.
2. **Dado** un archivo producido por Excel (campo `Application` de `docProps/app.xml`, sin señales de haberlo generado una biblioteca; `plan.md` §9.10), **cuando** el motor lo recalcula, **entonces** cada celda con fórmula se compara con el **valor que el archivo trae guardado** (números con tolerancia relativa 1e-9 o diferencia absoluta ≤ 1e-12; texto y booleanos exactos; errores del mismo tipo) y se registra M-09(b).
3. **Dado** un archivo producido por LibreOffice, Apache POI u otro productor distinto de Excel, o sin `docProps/app.xml`, **cuando** se procesa, **entonces** se abre y se mide (a) y (c), pero sus fórmulas **no** entran en (b) de SC-007: se informan aparte, porque su valor guardado no es el de Excel.
4. **Dado** un archivo cualquiera, **cuando** se abre, **entonces** los valores de las celdas sin fórmula son idénticos a los que lee un lector independiente del mismo archivo (M-09(c)).
5. **Dado** el conjunto de resultados, **cuando** termina la corrida, **entonces** el informe lista las funciones no soportadas (`#NAME?`) o con resultados distintos, ordenadas por frecuencia, con ejemplos (archivo, hoja, celda), y reporta todo **por fuente** (usuario / Apache POI / LibreOffice) además del total.
6. **Dado** un archivo con macros (`.xlsm`, incluidas macros XLM antiguas), **cuando** se procesa, **entonces** **ninguna macro se ejecuta nunca** y el archivo no se abre en Excel con macros habilitadas.
7. **Dado** archivos pequeños construidos a propósito con valores guardados conocidos (coincidentes y discrepantes), **cuando** se ejecuta la herramienta, **entonces** detecta exactamente las coincidencias y discrepancias esperadas (autoprueba de la herramienta).
8. **Dado** el informe, **cuando** se evalúa SC-007 y se aplica la regla del motor (sección 9), **entonces** el criterio queda como cumple / no cumple / sin datos con el dato que lo sustenta, y la decisión sobre el motor se registra sin mezclarla con la decisión de interfaz.

## 4. Casos límite

| Caso | Comportamiento esperado |
|---|---|
| Archivo corrupto o truncado, o con extensión `.xlsx` que no es un libro válido | Mensaje de error claro; sin cierre inesperado; el archivo no se modifica; la aplicación sigue utilizable. Si se abrió por línea de comandos, la automatización recibe una señal de error verificable. |
| Ruta inexistente, sin permiso de lectura o archivo de otro formato | Igual que el caso anterior. |
| Fórmulas con error (`#DIV/0!`, `#REF!`, nombre desconocido, `#VALUE!`) | Se muestra el valor de error en la celda; el error se propaga a las dependientes como en Office; nada se bloquea; `=SI.ERROR` lo captura. |
| Referencia circular | Se detecta y se informa; la aplicación no se cuelga ni entra en un bucle. |
| Función desconocida o sintaxis incorrecta al teclear | La celda muestra un error y el texto tecleado no se pierde; se puede corregir. |
| Archivo enorme (mucho mayor que A1) | La interfaz no se congela ni se cierra; indica progreso o rechaza el archivo con un mensaje claro si excede lo soportado. El umbral está en las aclaraciones. |
| Hoja vacía (libro nuevo o con hojas sin datos) | Se muestra la cuadrícula con cabeceras; flechas, Ctrl/Cmd+flechas (van al borde de la hoja), edición y guardado funcionan; guardar y reabrir conserva el libro vacío. |
| Celdas con texto muy largo (miles de caracteres, saltos de línea) | La celda recorta o desborda hacia las celdas vacías contiguas como en Office; no degrada M-03; la barra de fórmulas muestra el contenido completo; no se pierde texto al guardar. |
| Número que no cabe en el ancho de la columna | Se muestra como en Office (por ejemplo `####`) y no un valor recortado que parezca otro. |
| IME a mitad de composición al desplazar (rueda, trackpad, barras) | La composición no se pierde ni se confirma sola; la celda en edición conserva el texto en composición aunque salga del área visible, y la composición continúa al volver. |
| Intro durante una composición IME | Confirma la composición; la celda se confirma solo con una segunda pulsación. |
| Clic en otra celda durante una composición IME | Se confirma la composición y se aplica la edición de la celda. |
| Tecla muerta seguida de un carácter incompatible (`´` + `x`) | Se insertan ambos caracteres, como hace el sistema. |
| Teclas pulsadas durante un recálculo masivo (A2) | Ninguna se pierde y aparecen en el orden en que se pulsaron. |
| Deshacer tras un cambio con ≥10 000 dependientes | Restaura el valor y todos los dependientes. |
| Guardar sin permisos, con el disco lleno o con el archivo bloqueado por otra aplicación | Error claro; el archivo original no queda a medias ni corrupto. |
| Pantalla de alta densidad, escala distinta o cambio de monitor | Texto nítido y cuadrícula alineada; el informe declara la frecuencia de refresco de la pantalla usada en M-03. |
| Fuente del archivo no instalada | Se usa una fuente sustituta sin fallar. |
| Contenido no soportado en el archivo (gráficos, celdas combinadas, formato condicional…) | Se ignora al mostrar, sin fallar. Qué ocurre al guardar está en las aclaraciones. |
| Libro con macros (`.xlsm`, incluidas macros XLM antiguas) | Se abre como cualquier libro; **ninguna macro se ejecuta nunca**; se avisa al abrir que las macros no se conservan al guardar (aclaración 8). |
| Pegado con comillas, saltos de línea dentro de una celda o tabuladores finales | Se interpreta conforme al texto tabulado; sin cierres. |
| Hoja con cientos de miles de filas y barra arrastrada a cualquier punto | La posición de la barra corresponde a la fila mostrada; sin saltos ni desfases. |
| Archivo sin valores guardados en las fórmulas (por ejemplo, generado por una biblioteca que no calcula) | Se abre y cuenta en M-09(a) y (c); sus celdas con fórmula **no se comparan** (no hay valor de Excel con qué comparar), se cuentan aparte como «sin valor guardado» y no entran en el denominador de M-09(b). |
| Archivo producido por LibreOffice, Apache POI u otro productor distinto de Excel, sin `docProps/app.xml`, o que dice ser de Excel pero lo generó una biblioteca (`fullCalcOnLoad="1"` o sin valores guardados: «generador sospechoso», `plan.md` §9.10) | Se informa aparte, con su productor; sus fórmulas no entran en M-09(b) de SC-007 (aclaración 16). |
| Fórmulas volátiles que dependen del reloj o del azar (`HOY()`, `AHORA()`, `ALEATORIO()`, `ALEATORIO.ENTRE()`) | Su valor no puede coincidir con el guardado: se excluyen de M-09(b) y se cuentan aparte, junto con las celdas que dependen de ellas. No son discrepancias del motor. |
| Archivo del corpus que no es un libro válido, está cifrado o está dañado a propósito (los corpus públicos incluyen casos así) | No se bloquea la corrida; se registra el motivo y se informa por fuente. **Resuelto por defecto (2026-10-22):** si **ningún juez** (ONLYOFFICE con `x2t` y LibreOffice) puede abrirlo, se marca «inválido según los jueces» y se excluye del denominador de M-09(a); si algún juez lo abre, cuenta. Un cierre inesperado del motor cuenta **siempre**, sea cual sea el archivo. |

## 5. Requisitos

### 5.1 Requisitos funcionales

Las **dos interfaces deben ofrecer exactamente lo mismo**, para que la comparación sea justa.

| ID | Requisito | Historia | Cómo se comprueba |
|---|---|---|---|
| **FR-001** | Abrir un archivo `.xlsx` (diálogo de abrir y por argumento de línea de comandos, para automatizar mediciones). | US1 | Automático: abrir A1 por argumento y comprobar la primera pantalla. Manual: abrir A1 por diálogo y obtener el mismo resultado. |
| **FR-002** | Mostrar la cuadrícula: cabeceras de filas y columnas, valores formateados (formato de número) y **mostrar** estilos básicos existentes del archivo (negrita, cursiva, color de texto, color de relleno, alineación). Mostrar, no editar estilos. | US1 | Comparación visual lado a lado con ONLYOFFICE de una región de A1 con estilos variados y formatos de número distintos. |
| **FR-003** | Desplazamiento virtualizado y fluido (rueda, trackpad, barras) en hojas de 100 000+ filas: solo se dibuja lo visible. | US1 | M-03 sobre A1; la instrumentación muestra que las celdas dibujadas por cuadro no crecen con el tamaño de la hoja. |
| **FR-004** | Navegación y selección con teclado y ratón: flechas, Ctrl/Cmd+flechas (ir al borde), Mayús+flechas (rango), RePág/AvPág, clic y arrastre. | US1 | Pruebas funcionales con secuencias de teclas y ratón sobre A1 y sobre una hoja vacía. |
| **FR-005** | Editar celdas: escribir sobre la celda, editar en la barra de fórmulas, F2 (Windows/Linux) y su equivalente en macOS (Ctrl+U, como Excel para Mac; F2 también), Intro/Tab para confirmar, Esc para cancelar. | US2 | Pruebas funcionales con las secuencias de teclas de los escenarios 1–3 de US2. |
| **FR-006** | Fórmulas en **español e inglés** (`=SUMA`, `=BUSCARV`, `=BUSCARX`, `=SI.ERROR`…), con recálculo automático. | US2 | Pruebas con un conjunto fijo de fórmulas en ambos idiomas; resultado comparado con el esperado y con ONLYOFFICE; A2 se abre en español. |
| **FR-007** | Entrada de texto internacional: acentos y ñ, teclas muertas, IME (japonés/chino) y dictado del sistema. | US2 | Prueba manual guiada en las tres plataformas (SC-004); casos límite de IME de la sección 4. |
| **FR-008** | Deshacer / rehacer. | US2 | Pruebas funcionales: edición → deshacer → rehacer, incluyendo dependientes. |
| **FR-009** | Varias hojas: pestañas abajo para cambiar de hoja (crear/renombrar/borrar es opcional en la PoC). | US3 | Abrir A3 y comprobar cada pestaña. |
| **FR-010** | Copiar y pegar valores como texto tabulado, dentro de la aplicación y con otras aplicaciones (Excel/ONLYOFFICE). | US3 | Copiar de la aplicación a ONLYOFFICE/Excel y a la inversa; comparar el resultado. |
| **FR-011** | Guardar como `.xlsx`; el archivo debe reabrirse en ONLYOFFICE (y Excel si está disponible) sin errores ni pérdida de valores/fórmulas. | US3 | Comparación automática de ida y vuelta de A1–A3 (SC-005) más apertura manual en ONLYOFFICE. |
| **FR-012** | Instrumentación de medidas integrada (local, sin red), activable por bandera, que escribe resultados en JSON. | US4 | Con la bandera: aparece el JSON; sin ella: no se escribe nada; con la red deshabilitada el resultado es el mismo. |
| **FR-013** | Generador de archivos de prueba `.xlsx` sintéticos y reproducibles (semilla fija). | US4 | Dos ejecuciones con la misma semilla producen el mismo contenido celda a celda; A1–A3 cumplen la sección 7. |
| **FR-014** | Herramienta de compatibilidad (y el informe que produce) que, para cada archivo de A4: (1) lo abre con el motor de la hoja y registra error o cierre inesperado; (2) recalcula; (3) compara cada celda con fórmula con el **valor que el archivo trae guardado**, **solo** en archivos producidos por Excel (campo `Application` de `docProps/app.xml`; los de LibreOffice, Apache POI u otros se informan aparte) y excluyendo las funciones volátiles, que se cuentan aparte; (4) compara los valores sin fórmula tras abrir; (5) lista las funciones no soportadas (`#NAME?`) o con resultados distintos, ordenadas por frecuencia; (6) **nunca ejecuta macros**; (7) escribe un JSON y una tabla, por archivo y por fuente. Usa un lector independiente del motor para los valores guardados y las fórmulas. | US5 | Autoprueba con archivos pequeños de valor guardado conocido (coincidentes y discrepantes); corrida sobre A4 con el JSON resultante (M-09). |

### 5.2 Entidades clave

- **Libro**: conjunto de hojas; es lo que se abre y se guarda como `.xlsx`.
- **Hoja**: cuadrícula de celdas con nombre; hay una hoja activa.
- **Celda**: contenido (valor o fórmula), valor mostrado con formato de número y estilo básico (negrita, cursiva, color de texto, color de relleno, alineación).
- **Selección**: celda activa y, opcionalmente, un rango.
- **Informe de medidas**: JSON con las medidas M-xx de una corrida (aplicación, plataforma, hardware, repeticiones, mediana, p95).
- **Archivo de prueba**: A1–A4 de la sección 7.
- **Informe de compatibilidad**: JSON y tabla con M-09 por archivo, por productor (Excel / LibreOffice / Apache POI / otro) y por fuente (usuario / Apache POI / LibreOffice), con la lista de funciones faltantes o distintas.

## 6. Medidas

Cada medida se toma **5 veces**, con mediana y p95, **en la misma máquina**, en frío y en caliente cuando aplique. Se miden las tres aplicaciones (interfaz A, interfaz B y ONLYOFFICE 10.0.0) con el **mismo método externo** y, además, las dos interfaces con su instrumentación interna. Los números internos sirven para diagnosticar y **no se mezclan** con los externos al comparar contra ONLYOFFICE. El método exacto lo define `plan.md` y debe ser justo con ONLYOFFICE (ver `research.md`).

*Frío* = sin que la aplicación ni el archivo se hayan usado antes en esa máquina de forma reciente. *Caliente* = inmediatamente después de una ejecución previa.

**Excepción: M-09** (compatibilidad) no mide tiempo: es determinista, se calcula una vez por archivo y no usa repeticiones, mediana ni p95 (se repite solo para confirmar que el resultado no cambia entre corridas). Se reporta por archivo, por fuente y en total.

| ID | Qué mide | Unidad | Cómo se interpreta | Frío / caliente |
|---|---|---|---|---|
| **M-01** | Arranque en frío hasta ventana usable | ms | Desde lanzar el proceso hasta que la ventana acepta entrada y muestra la cuadrícula. Menos es mejor. Qué cuenta como «usable» en ONLYOFFICE (pantalla de inicio o editor de hojas listo) lo fija el plan y se declara en el informe. | Frío y caliente |
| **M-02** | Abrir archivo grande hasta la primera pantalla con datos | ms | Desde pedir abrir A1 hasta el primer cuadro con datos visibles y correctos (no marcadores de carga). Menos es mejor. | Frío y caliente |
| **M-03** | Fluidez al desplazar: tiempo por cuadro durante 10 s de desplazamiento continuo, y cuadros perdidos | ms (p95) y número de cuadros perdidos | Menos es mejor. 16,7 ms equivale a 60 Hz y 8,3 ms a 120 Hz. Un cuadro perdido es el que supera el intervalo de refresco de la pantalla usada, que se declara. | Caliente |
| **M-04** | Latencia de tecleo: tecla → carácter visible en la celda | ms (p95) | Menos es mejor. En la medición externa incluye el camino del sistema operativo; la interna empieza en la recepción del evento. | Caliente |
| **M-05** | Recálculo: cambiar una celda de la que dependen ≥10 000 fórmulas → valores visibles actualizados | ms | Menos es mejor. Se mide sobre A2. | Caliente |
| **M-06** | Memoria total, suma de **todos** los procesos de la aplicación (incluidos los auxiliares de renderizado o GPU), con el archivo grande abierto | MB | Menos es mejor. La métrica concreta de memoria la fija el plan y es la misma para las tres aplicaciones. Se mide con A1. | Caliente |
| **M-07** | Guardar `.xlsx` + verificación de integridad (FR-011) | ms | Menos es mejor. **Solo vale si la verificación pasa**: un guardado rápido con pérdida se reporta como fallo. Tiempo con A1; integridad con A1–A3. | Frío y caliente |
| **M-08** | Tamaño de la aplicación empaquetada | MB | Menos es mejor. La línea base de ONLYOFFICE incluye procesador de textos y presentaciones, así que su alcance es mayor que el de la PoC; se informa con esa salvedad, sin ajustar el dato. | No aplica |
| **M-09** | Compatibilidad del motor con archivos reales (A4): (a) % de archivos que abren sin error y número de cierres inesperados; (b) % de celdas con fórmula cuyo valor recalculado coincide con el guardado por Excel; (c) % de celdas sin fórmula idénticas tras abrir; (d) lista de funciones faltantes o distintas, ordenada por frecuencia | % y lista | Más es mejor en (a), (b), (c); 0 en cierres. (b): números con tolerancia relativa 1e-9, texto y booleanos exactos, errores del mismo tipo; solo archivos producidos por Excel; se excluyen las funciones volátiles (se cuentan aparte) y las celdas sin valor guardado. Se informa por fuente (usuario / Apache POI / LibreOffice) y en total. | No aplica |

**Agregación (propuesta).** Con 5 repeticiones, el p95 es prácticamente el peor valor. Para M-03 y M-04 se calcula primero el p95 de todas las muestras de cada repetición (cuadros o teclas) y se reporta la mediana de esos cinco p95 junto con el peor. Para M-01, M-02, M-05, M-07 se reporta la mediana y el p95 de las cinco repeticiones, con el mínimo y el máximo. Ver aclaración 10. M-09 no se agrega con mediana ni p95 (ver arriba).

## 7. Archivos de prueba

Los define el generador (FR-013). Todos se producen con semilla fija, salvo A4.

| ID | Descripción | Se usa en |
|---|---|---|
| **A1** | 100 000 filas × 20 columnas: números, textos y fechas, con estilos básicos variados (negrita, cursiva, color de texto, color de relleno, alineación). | US1, M-01…M-04, M-06, M-07 |
| **A2** | Igual que A1 más ≥10 000 fórmulas encadenadas que dependen de una celda; las fórmulas están en español al abrir con el idioma es. | US2, M-05, SC-005 |
| **A3** | Libro con 5 hojas medianas (10 000 filas cada una). | US3, FR-009, SC-005 |
| **A4** | Archivos reales: los del usuario más corpus públicos con licencia compatible (archivos de prueba de Apache POI, Apache-2.0: 353 `.xlsx` y 14 `.xlsm` con macros; y de LibreOffice, MPL-2.0: 312 `.xlsx`), seleccionados con un manifiesto que registra fuente, licencia y hash. Se guardan solo en local y **nunca se suben al repositorio**. Los `.xlsm` sirven para comprobar que la aplicación los abre, **no ejecuta nada** y avisa que las macros no se conservan. | US5, FR-014, M-09, SC-007. **Alimenta SC-007** (ya no es solo informativa) y se informa **por fuente** (usuario / Apache POI / LibreOffice): los corpus de POI y LibreOffice incluyen muchos archivos raros o de reportes de errores, no representativos del uso diario, así que los archivos del usuario pesan más al interpretar el resultado. |

## 8. Criterios de éxito (propuesta)

**Estado: propuesta, ajustable antes de medir.** Los umbrales se confirman antes de la primera corrida oficial (Constitución, principio VIII). SC-001…SC-006 se evalúan **por plataforma**; SC-007 mide el motor y se evalúa sobre A4 (no depende de la interfaz). Lo no medido es «sin datos».

| ID | Criterio | Medidas / requisitos | Cómo se comprueba |
|---|---|---|---|
| **SC-001** | Al menos una interfaz supera a ONLYOFFICE en M-02 por **≥2×** (la mediana de la interfaz es ≤ la mitad de la de ONLYOFFICE) y en M-06 usa **≤50 %** de su memoria. | M-02, M-06 | Tabla comparativa de la misma máquina con el mismo método externo. |
| **SC-002** | M-03: p95 ≤ 16,7 ms (60 Hz) en las tres plataformas; ideal ≤ 8,3 ms (120 Hz) en macOS con ProMotion si hay pantalla disponible. | M-03 | Medición externa y registro de la frecuencia de refresco de la pantalla. |
| **SC-003** | M-04: p95 ≤ 30 ms. | M-04 | Medición externa de tecla → carácter visible. |
| **SC-004** | FR-007 pasa en las tres plataformas: prueba manual guiada con á, ñ, ü, japonés con IME y dictado en macOS. | FR-007 | Guion de prueba manual con resultado por plataforma. |
| **SC-005** | FR-011: ida y vuelta de A1–A3 sin diferencias de valores ni de fórmulas. | FR-011, M-07 | Comparación automática de las celdas del archivo original y del reabierto. |
| **SC-006** | Ambas interfaces compilan y pasan las pruebas funcionales en integración continua para las tres plataformas. | Todos los FR | Ejecución de la integración continua en macOS, Windows y Linux (no mide rendimiento). |
| **SC-007** | Compatibilidad del motor con archivos reales (A4): (a) **≥ 95 %** de los archivos abren sin error y **0** cierres inesperados; (b) **≥ 98 %** de las celdas con fórmula coinciden con el valor guardado por Excel, en archivos producidos por Excel; (c) **100 %** de los valores sin fórmula idénticos tras abrir. Se informa también **por fuente** (usuario / Apache POI / LibreOffice). | M-09, FR-014 | Corrida de la herramienta de compatibilidad sobre A4 (sin ejecutar macros). Mide el motor, no la interfaz: no depende de cuál interfaz se elija. Sin A4 preparado queda «sin datos». |

## 9. Regla de decisión Tauri vs GPUI

Es la única sección, además de «Supuestos, restricciones y dependencias», que nombra las interfaces: **Tauri** es la interfaz A y **GPUI** es la interfaz B. La regla se fija **antes** de medir.

Una interfaz **cumple** si satisface SC-001…SC-004 en las plataformas con medición en hardware real. Por decisión del dueño (2026-10-22, aclaración 2) por ahora solo se mide en **macOS**: la decisión que salga de la regla es **definitiva para macOS y provisional para Windows y Linux**, que solo tienen pruebas funcionales en CI. Antes de iniciar la fase 1 en esos sistemas hay que medirlos en hardware real (idealmente Linux con NVIDIA) y volver a aplicar esta regla.

| Orden | Condición | Decisión |
|---|---|---|
| 1 | Ninguna interfaz supera a ONLYOFFICE (ninguna cumple SC-001). | **Se detiene** la hoja de ruta y se revisa la arquitectura antes de la fase 1. |
| 2 | Tauri cumple y GPUI también, pero GPUI es mucho mejor (≥2× en M-03/M-04). | **Decisión del usuario** con los datos. |
| 3 | Tauri cumple (en cualquier otro caso). | Se elige **Tauri**: menos trabajo de interfaz y más fácil lograr el aspecto de Office/ONLYOFFICE con HTML/CSS en la fase 1. |
| 4 | Tauri falla (sobre todo en M-03 o en Linux con NVIDIA) y GPUI cumple. | Se elige **GPUI**. |
| — | Al menos una interfaz supera a ONLYOFFICE (SC-001) pero ninguna cumple SC-002…SC-004; o duda sobre la lectura de «≥2× en M-03/M-04». | No cubierto por la regla: ver aclaración 9. |

SC-007 **no interviene** en esta regla: la decisión de interfaz sigue únicamente SC-001…SC-004.

### Regla del motor (SC-007)

**SC-007 decide sobre el motor de la hoja (IronCalc, sección 11), no sobre la interfaz.** Se aplica por separado de la regla anterior y tiene su propio resultado en el informe. Se fija **antes** de medir.

| Orden | Condición | Decisión |
|---|---|---|
| M1 | SC-007 cumple (a), (b) y (c). | El motor queda validado para iniciar la fase 1. |
| M2 | SC-007 no cumple y (b) ≥ 90 %. | **Antes de la fase 1**: plan de motor que use la lista de funciones faltantes o distintas (M-09(d)) para contribuir a IronCalc (su licencia lo permite) y volver a medir. |
| M3 | (b) < 90 %. | Además del plan de M2, **evaluar alternativas de motor de fórmulas** antes de la fase 1. |
| — | Sin datos (A4 sin preparar) o sin archivos producidos por Excel en A4. | SC-007 queda «sin datos»: el motor **no** se da por validado. |

La regla del motor puede detener o condicionar la fase 1 aunque la regla de interfaz haya elegido una interfaz.

## 10. Fuera de alcance

Cinta (ribbon) · editar formatos o estilos (solo se muestran) · gráficos · imágenes · celdas combinadas · formato condicional · filtros · ordenar · validación de datos · tablas dinámicas · macros · impresión / PDF · procesador de textos · presentaciones · colaboración · autocompletado de fórmulas (opcional solo si sale gratis del motor) · instaladores firmados o notarizados · accesibilidad completa (se documenta como riesgo; la entrada con IME **sí** está dentro) · versión web.

Crear, renombrar y borrar hojas es **opcional** en la PoC (FR-009).

## 11. Supuestos, restricciones y dependencias

*Aquí sí se nombran tecnologías. Las razones y fuentes están en `research.md`.*

**Interfaces candidatas**
- **Interfaz A:** Tauri 2 estable (2.12.5 a 2026-10-21) con la webview del sistema.
- **Interfaz B:** GPUI (nativa con GPU) como dependencia git fijada a un commit de Zed (la versión de crates.io, 0.2.2, es del 22-oct-2025).
- Ambas sobre el mismo núcleo en Rust, sin depender de ninguna interfaz.

**Restricciones**
- El motor de hoja de cálculo es IronCalc (0.9.1, pre-1.0; licencia MIT OR Apache-2.0). Es solo la calculadora y el lector/escritor `.xlsx` de la hoja; la suite tiene otros motores por tipo de documento (`../000-arquitectura-suite/arquitectura.md`).
- La compatibilidad (FR-014, M-09) usa un lector independiente del motor: `calamine` 0.37.0 (MIT), para los valores guardados y las fórmulas. Los archivos de prueba pequeños de la herramienta se construyen con `rust_xlsxwriter`, que permite fijar el valor guardado de una fórmula.
- El formato nativo es `.xlsx`.
- Licencia del proyecto MIT OR Apache-2.0; no se incorpora código GPL, LGPL ni AGPL, ni código de ONLYOFFICE (Constitución, principios I y VII).
- Repositorio público: https://github.com/jahziel1/dokumentz
- Sin red, sin cuentas, sin telemetría; las mediciones son locales (FR-012).
- Tauri 3 con CEF (alfa) no forma parte de la PoC: queda solo como salida de emergencia documentada.
- La integración continua (GitHub Actions) compila y ejecuta pruebas funcionales; **no** mide rendimiento.

**Máquina del usuario**
- Apple M1, 16 GB de RAM, macOS 26.6.2; rustc 1.99.0, cargo 1.99.0, Node 22.23.3, Xcode instalado.
- **El usuario solo tiene Mac.** Para medir Windows y Linux hace falta **hardware real**, e **idealmente un Linux con NVIDIA** para evaluar el riesgo de la webview en esa combinación. Las máquinas virtuales sirven para pruebas funcionales, no para medir rendimiento por falta de GPU.

**Línea base**
- ONLYOFFICE 10.0.0 instalado y analizado en el Mac del usuario (binario arm64, 941 MB, editores en JavaScript sobre Chromium Embedded Framework). Los datos están en `research.md`.
- ONLYOFFICE y LibreOffice sirven también como **jueces de compatibilidad**.

**Dependencias de versión de terceros**
- macOS: la versión de WebKit depende de la versión de macOS.
- Windows: WebView2 se actualiza solo; existe la opción de versión fija.
- Linux: WebKitGTK depende de la distribución; hay problemas conocidos con NVIDIA y mitigaciones por variables de entorno que no deben aplicarse a todos.
- Excel es opcional (solo «si está disponible»).
- Los archivos A4 nunca salen del equipo. Ya no son opcionales para SC-007: sin A4 preparado, SC-007 queda «sin datos» (los criterios SC-001…SC-006 no dependen de A4).
- Descargar los corpus públicos de A4 requiere el visto bueno del dueño (aclaración 12).

## 12. Aclaraciones

La numeración se conserva porque el resto del documento remite a ella.

### Resueltas por el dueño (2026-10-22)

1. **Licencia:** MIT OR Apache-2.0 (Constitución v1.1.0, principio I). No se admiten dependencias GPL, LGPL ni AGPL.
2. **Hardware:** por ahora **solo el Mac** del usuario (MacBook Air M1, `MacBookAir10,1`, pantalla integrada Retina de 60 Hz, sin ProMotion; SC-002 se evalúa con el objetivo de 60 Hz). Windows y Linux solo tienen pruebas funcionales en CI; las tareas de medición en esos sistemas y la de 120 Hz siguen bloqueadas hasta conseguir equipo, y la decisión de interfaz es provisional para ellos (sección 9).
12. **A4:** el usuario aporta archivos propios (los copia a la carpeta local de A4; ruta por confirmar al preparar la etapa) y se suman corpus públicos con licencia compatible, incluidos libros con macros (sección 7). Descargar los corpus requiere el visto bueno del dueño con la lista de archivos, fuente y tamaño. Los archivos de internet se tratan como no confiables: la aplicación no ejecuta macros y nadie los abre en Excel con macros habilitadas.
13. **Repositorio:** GitHub público, https://github.com/jahziel1/dokumentz (vacío al 2026-10-22). Inicializar git y subir el primer cambio se hace en CP-0, con confirmación del dueño.
14. **EX-01:** aprobada por el dueño (plan.md §3); caduca al cerrar la fase 0.
15. **Arquitectura de la suite (2026-10-22):** el dueño aprobó la suite completa con la hoja de ruta 0a/0b/1…5 de [`../000-arquitectura-suite/arquitectura.md`](../000-arquitectura-suite/arquitectura.md), la incorporación de US5/FR-014/M-09/SC-007 a esta PoC y los nombres de producto (`dokumentz-*`). Queda pendiente allí la confirmación de aplicación única frente a tres aplicaciones.

### Resueltas por defecto (2026-10-22; se pueden cambiar registrando el cambio aquí)

3. **ONLYOFFICE en Windows y Linux:** se mide la versión estable más reciente disponible en cada plataforma el día de la corrida, y se registra en el informe. SC-001 se evalúa por plataforma, solo donde haya línea base medida.
4. **Editar celda en macOS:** Ctrl+U, como Excel para Mac; F2 (o fn+F2) también entra en edición.
5. **Idioma de las fórmulas:** es un ajuste de la aplicación (por defecto, el idioma del sistema). Las fórmulas se muestran en el idioma activo y, al teclear, se aceptan también los nombres del otro idioma (`=SUM` funciona con el español activo y se muestra como `=SUMA`). Las corridas oficiales se hacen en español y A2 se abre en español.
6. **Copiar:** como Excel, el texto plano copiado es el valor **tal como se ve** en la celda (con su formato de número). Dentro de la aplicación, pegar conserva el valor original.
7. **Archivo enorme:** hasta el límite de Excel (1 048 576 filas × 16 384 columnas), la aplicación no se congela y muestra progreso al abrir; si no cabe en memoria, muestra un mensaje claro y no se cierra. Prueba de estrés opcional (no oficial): 1 000 000 filas × 20 columnas.
8. **Contenido no soportado:** al abrir, la aplicación avisa si el archivo tiene elementos que la PoC no conserva (gráficos, celdas combinadas, formato condicional…). Al guardar, propone un nombre nuevo y nunca sobrescribe el original sin confirmación.
9. **Huecos de la regla de decisión:** si al menos una interfaz cumple SC-001 pero ninguna cumple SC-002…SC-004, decide el usuario con los datos y con un diagnóstico de qué falló. «≥2× en M-03/M-04» significa en **al menos una** de las dos medidas.
10. **Agregación:** se adopta la propuesta de la sección 6; frío y caliente según la tabla de medidas.
11. **Umbrales:** se adoptan los propuestos en la sección 8. Pueden ajustarse antes de la primera corrida oficial, nunca después de ver los datos.
16. **Umbrales y alcance de SC-007:** se adoptan como propuesta los de la sección 8 ((a) ≥ 95 % de archivos sin error y 0 cierres; (b) ≥ 98 % de fórmulas coinciden con Excel; (c) 100 % de valores sin fórmula idénticos), con la misma regla que SC-001…SC-006: pueden ajustarse antes de la primera corrida oficial, nunca después de ver los datos. (b) se calcula solo sobre archivos producidos por Excel; los de LibreOffice, Apache POI u otro productor, o sin `docProps/app.xml`, se informan aparte. Las funciones volátiles se excluyen de (b) y se cuentan aparte. Todo se informa por fuente (usuario / Apache POI / LibreOffice).
