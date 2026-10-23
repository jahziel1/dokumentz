# Arquitectura de la suite Dokumentz

## 1. Ficha del documento

| Campo | Valor |
|---|---|
| Proyecto | **Dokumentz** (https://github.com/jahziel1/dokumentz) |
| Versión | 1.0 |
| Fecha | 2026-10-22 |
| Estado | **Aprobado por el dueño** (decisiones cerradas el 2026-10-22; lo no decidido va marcado `[NECESITA ACLARACIÓN: ...]`) |
| Relación con la constitución | Subordinado a `specs/constitucion.md` (v1.1.1). Desarrolla sus principios III (regla de oro), IV (formatos nativos) y VIII (medir antes de decidir) a escala de suite. La hoja de ruta de la sección 8 es la que cita la constitución desde la v1.1.1 (enmienda PATCH aprobada por el dueño el 2026-10-22) |
| Relación con la spec 001 | `specs/001-poc-hoja-de-calculo/` es la fase 0a. Sus nombres ya son los de producto (`dokumentz-sheet`, `dokumentz-metrics`, `dokumentz-script`, `testdata-gen`, `dokumentz-app-tauri`/`-gpui`). Agrega SC-007 (compatibilidad) y la herramienta `tools/compat-check`. Su excepción EX-01 se cierra cuando nace `dokumentz-text` (sección 5) |
| Relación con la spec 002 | `specs/002-poc-documentos/` es la fase 0b. **Aún no existe**: se escribe cuando la 001 pase su CP-1. La sección 9 es solo su esbozo |
| Convenciones | `[VERIFICADO]` = comprobado el 2026-10-22 con la URL indicada. `[HIPÓTESIS]` = razonable pero sin medir (principio VIII: se mide antes de apoyar trabajo grande en ella). IDs de las specs (FR, M, SC, CP, D-xx, EX-xx) se citan sin renumerar |

---

## 2. Por qué una arquitectura de suite

El dueño quiere la **suite completa** (hoja, documentos y presentaciones) como reemplazo real de Office, no una PoC suelta. La primera planificación tenía cuatro fallas que este documento corrige:

1. La PoC se pensaba como instrumento desechable; ahora sus piezas de producto se quedan (sección 7).
2. IronCalc se veía como «el motor». Es solo la calculadora de la hoja (sección 6).
3. Lo más difícil de la suite, **paginar documentos con la fidelidad de Word**, quedaba para el final. Ahora se prueba en la fase 0b, antes de construir nada grande encima.
4. La PoC medía velocidad e interfaz, pero no el motivo original del proyecto: las alternativas **no abren bien los archivos de la gente**. Además: LibreOffice se siente viejo y descuadra; Google lo logra, pero en la nube; hay que cubrir el día a día (incluidas las macros); y debe sentirse como Office (como ONLYOFFICE), pero rápido.

### Hechos que justifican el diseño

| Hecho | Consecuencia |
|---|---|
| Ninguna suite es «un solo motor». Microsoft Office tiene motores distintos para Excel, Word y PowerPoint sobre una base común; LibreOffice tiene Calc, Writer e Impress sobre módulos comunes | Dokumentz = **base común + un motor por tipo de documento**, no un motor universal |
| En el ONLYOFFICE instalado en el Mac del dueño, `sdkjs/common` pesa 37 MB, más que `word` (10 MB) y `cell` (11 MB) juntos (medición local, 2026-10-22) | Lo compartido es la parte más grande: se diseña primero y una sola vez |
| Los únicos motores **completos** que existen son LibreOffice (fidelidad y aspecto que no convencieron; C++ enorme; MPL-2.0 + LGPL; LibreOfficeKit entrega teselas con API experimental, D-05) y ONLYOFFICE (JavaScript, lento; AGPL) | No se puede reutilizar uno completo sin violar la constitución (principios I y VII) o la razón de ser (velocidad, aspecto) |
| IronCalc 0.9.1: MIT OR Apache-2.0 (verificado en crates.io y en sus archivos `LICENSE-MIT` y `LICENSE-Apache-2.0`); su tabla de idiomas lista 497 nombres de función (Excel tiene alrededor de 500); no se sabe cuántas calculan igual que Excel; reevalúa todo el libro en cada edición (verificado en v0.9.1) | Sirve para la hoja, pero su compatibilidad se **mide** (SC-007) y no se da por supuesta |
| Licencia del proyecto: MIT OR Apache-2.0; prohibidas las dependencias GPL, LGPL y AGPL (constitución v1.1.0) | Cada candidato de motor se filtra por licencia antes de evaluarlo |

**Conclusión.** No existe un motor completo que sea a la vez rápido, fiel, local y permisivo. Ese es el hueco que llena Dokumentz, y solo se llena construyendo (a) una base común propia, (b) un motor por tipo de documento sobre ella y (c) midiendo en cada fase la velocidad **y** la compatibilidad con archivos reales. Fuentes de IronCalc y LibreOfficeKit: `research.md` (D-02, D-05).

---

## 3. Principios de arquitectura derivados de la constitución

| # | Principio | Qué exige a la arquitectura | Cómo se verifica |
|---|---|---|---|
| A1 | **Regla de oro** (constitución III) | La interfaz solo dibuja y recibe entradas. Modelo, fórmulas, acomodo de texto, fuentes, archivos, portapapeles con formato, PDF e impresión viven en crates de Rust que no dependen de ninguna interfaz. Con tres apps esto es más estricto: la interfaz **ni siquiera conoce el tipo de documento**; solo habla `dokumentz-edit` (entradas) y `dokumentz-render` (dibujo) | Prueba `no_ui_deps` de la 001 ampliada a todos los crates fuera de `dokumentz-shell` y `apps/*`; cada crate nuevo se prueba sin ventana; revisión del diff |
| A2 | **Base común antes que soluciones por app** | Si dos apps van a necesitar algo (ZIP/relaciones, texto, formas, imágenes, portapapeles), se escribe en un crate de la base común, aunque la primera en usarlo sea una sola. Prohibido copiar código entre `dokumentz-sheet`, `-doc` y `-slide` | Pregunta fija en cada revisión: «¿lo necesitará otra app?». Si sí y está dentro de un motor, se mueve a la base antes de aceptar el cambio |
| A3 | **Conservar lo que no se entiende al guardar** (constitución IV) | Abrir y guardar no puede destruir contenido que Dokumentz aún no soporta: macros (`vbaProject.bin`), gráficos, formas, propiedades, partes de otros fabricantes. `dokumentz-opc` guarda las partes desconocidas tal cual y las vuelve a escribir | Prueba de ida y vuelta: toda parte que ningún motor modela sale byte a byte igual; `compat-check` y las pruebas de la fase 1 lo registran |
| A4 | **Medir velocidad y compatibilidad en cada fase** (constitución II y VIII) | Ninguna fase termina sin los dos informes: velocidad (`dokumentz-metrics`, `bench-runner`, `bench-report`) y compatibilidad (`compat-check` y su equivalente para documentos). Las reglas de decisión se escriben **antes** de medir | Cada fase tiene su compuerta (sección 8); sin los dos JSON no hay cierre |
| A5 | **Formatos de Office nativos** (constitución IV) | `.xlsx`, `.docx` y `.pptx` son los formatos de trabajo. No hay formato propio ni paso de «exportar». «Guardar» escribe el formato Office | Ida y vuelta abrir → guardar → reabrir; jueces ONLYOFFICE y LibreOffice (principio VII), Excel/Word/PowerPoint si hay |
| A6 | **Las dependencias van solo hacia abajo** | Capas: interfaz → edición → motores → base común. Ningún crate de la base conoce a un motor; ningún motor conoce a otro motor. Un motor se puede reemplazar (plan B de la sección 9) sin tocar las capas de arriba | Grafo de dependencias (`cargo tree`) revisado en CI; `cargo deny` para licencias |

---

## 4. Mapa de capas y módulos

### 4.1 Diagrama

```
 ┌─────────────────────────────────────────────────────────────────────────────┐
 │ apps/hoja · apps/documentos · apps/presentaciones                           │
 │   TRES apps, como Excel, Word y PowerPoint; un solo instalador (ver 4.4)    │
 ├─────────────────────────────────────────────────────────────────────────────┤
 │ dokumentz-shell       INTERFAZ (Tauri o GPUI, la que gane la PoC 001)       │
 │   ventana · pestañas · cinta · menús · diálogos · atajos por sistema        │
 │   SOLO dibuja y recibe entradas (teclado, ratón, IME, dictado)              │
 ├────────────────────────────┬────────────────────────────────────────────────┤
 │   comandos ▼               │            ▲ lista de dibujo (binario)         │
 ├────────────────────────────┴────────────────────────────────────────────────┤
 │ dokumentz-edit        comandos genéricos · deshacer/rehacer · selección ·   │
 │                       portapapeles con formatos (texto, TSV, HTML, imagen)  │
 │                       enruta cada comando al motor del documento activo     │
 ├──────────────────────┬──────────────────────┬───────────────────────────────┤
 │ dokumentz-sheet      │ dokumentz-doc        │ dokumentz-slide               │
 │  hoja: IronCalc para │  documentos: modelo  │  presentaciones: diapositivas,│
 │  fórmulas y .xlsx    │  .docx + paginación  │  modo presentador (fase 3)    │
 │  (PoC 001)           │  (PoC 002 → fase 2)  │                               │
 ├──────────────────────┴──────────────────────┴───────────────────────────────┤
 │ BASE COMÚN (la usan los tres motores)                                       │
 │  dokumentz-text     fuentes · shaping · saltos de línea · bidi · párrafos   │
 │  dokumentz-drawing  DrawingML: temas, colores, formas, imágenes, gráficos   │
 │  dokumentz-opc      paquete Office: ZIP · [Content_Types].xml · relaciones  │
 │                     · partes (conserva las que no entiende)                 │
 │  dokumentz-render   protocolo de dibujo genérico + formato binario          │
 ├─────────────────────────────────────────────────────────────────────────────┤
 │ TRANSVERSALES   dokumentz-metrics (medición)   dokumentz-script (guiones)   │
 └─────────────────────────────────────────────────────────────────────────────┘
 tools/  testdata-gen · xlsx-diff · compat-check · bench-runner · bench-report
         (viven fuera de la app; usan los crates del núcleo, nunca la interfaz)
```

Sentido de las dependencias entre crates (A6): `shell → edit → {sheet, doc, slide} → {text, drawing, opc, render}`; dentro de la base: `drawing → {opc, text, render}`, `text → render`, `opc` y `render` son hojas. `metrics` es opcional por característica en todos. Ningún crate de la base ni de los motores depende de `shell` ni de `apps/*`.

### 4.2 Tabla por crate

«Estado»: **PoC 001** = especificado en la 001 y se construye allí (a 2026-10-22 aún no hay código, solo especificaciones); **Por construir** = nace en la fase indicada.

| Crate | Responsabilidad | Qué NO hace | Depende de | Nace en | Estado |
|---|---|---|---|---|---|
| `dokumentz-opc` | Paquete Office: ZIP, `[Content_Types].xml`, relaciones, partes. Lee y escribe cualquier paquete OOXML. **Conserva las partes que no entiende** (macros `vbaProject.bin`, gráficos aún no soportados…) | No interpreta hojas, párrafos ni diapositivas; no ejecuta macros jamás | Solo crates de ZIP/XML | Fase 1 | Por construir |
| `dokumentz-drawing` | DrawingML compartido: temas, colores, formas, imágenes y gráficos (el mismo vocabulario en `.xlsx`, `.docx` y `.pptx`). Los convierte en primitivas de `dokumentz-render` | No decide dónde va un gráfico en una hoja o en una página (eso es del motor); no acomoda texto por sí mismo | `opc`, `text`, `render` | Fases 1–4 (crece por tipo de objeto) | Por construir |
| `dokumentz-text` | **Motor de texto común**: carga de fuentes, shaping, saltos de línea, bidireccional, párrafos. Lo usan celdas, páginas y diapositivas. La PoC 002 decide con qué candidatos se construye (sección 9). Al entrar, **se cierra EX-01** | No dibuja (entrega glifos ya acomodados al protocolo de dibujo); no conoce estilos de Office | `render` + bibliotecas de fuentes elegidas | Prototipo en la 0b; producto en fase 1 | Por construir |
| `dokumentz-render` | Protocolo de dibujo genérico (rectángulos, texto ya acomodado, líneas, imágenes, recortes) y su formato binario. Lo consumen las tres apps a través de la misma interfaz | No contiene lógica de documento; no sabe qué es una celda o un párrafo | Nada interno | Fase 1 | Por construir. La lista de dibujo de la cuadrícula (`drawlist`, formato «PDL1», en `dokumentz-sheet`) es específica de la hoja y se convierte en su primer cliente |
| `dokumentz-edit` | Comandos genéricos, deshacer/rehacer, selección, portapapeles con formatos (texto, TSV, HTML, imagen). Mantiene las sesiones de documento abiertas (una por ventana, como Office) y enruta cada comando al motor del documento | No interpreta teclas (eso es de la interfaz); no implementa operaciones propias de un tipo de documento | `sheet`, `doc`, `slide`, `render` | Fase 1 | Por construir (en la 001 los comandos viven en `dokumentz-sheet`, `command.rs`) |
| `dokumentz-metrics` | Instrumentación: `Recorder`, `Span`, `Histogram`, JSON de resultados (esquema `dokumentz-bench/1`). Apagada por defecto, solo escribe un archivo local | Nada de red ni telemetría (principio I) | — | PoC 001 | PoC 001 (permanente, compartida) |
| `dokumentz-script` | Guiones de prueba compartidos (`Step`, `Script`, `ScriptResult`) y el binario `dokumentz-script-check`. Garantiza que las interfaces hacen lo mismo | No conoce ninguna interfaz concreta | — | PoC 001 | PoC 001 (permanente) |
| `dokumentz-sheet` | Motor de la hoja: envuelve IronCalc (`UserModel`) para fórmulas y `.xlsx`; geometría, vista de cuadrícula, navegación, edición, comandos, TSV | No hace gráficos, macros ni formas; no acomoda texto (hoy lo hace la interfaz por EX-01; mañana `dokumentz-text`) | PoC 001: `ironcalc*`, `serde`, `thiserror`, `metrics`. Fase 1: + `opc`, `text`, `render`, `drawing` | PoC 001 | PoC 001 |
| `dokumentz-doc` | Documentos: modelo de `.docx` (WordprocessingML) y **paginación**. En la 0b solo lectura y dibujo; en la fase 2 edita y guarda | No hace formato de hoja ni diapositivas; no ejecuta macros | `opc`, `drawing`, `text`, `render` (+ el candidato que elija la 0b) | PoC 002 (0b) | Por construir |
| `dokumentz-slide` | Presentaciones: diapositivas, texto, imágenes, formas, modo presentador | No hace paginación de flujo; no hace animaciones hasta la fase 4 | `opc`, `drawing`, `text`, `render` | Fase 3 | Por construir |
| `dokumentz-shell` | Biblioteca de interfaz común a las tres apps: ventana de documento, cinta (con revisión legal previa, principio V), menús, diálogos, atajos por sistema, pantalla de inicio y archivos recientes. En la tecnología que gane la PoC 001 | No calcula, no acomoda texto, no lee archivos, no formatea números, no sabe de tipos de documento | `edit`, `render` (decodificar), `metrics` | Fase 1 | Por construir (semilla: la interfaz ganadora de la 001) |
| `apps/hoja`, `apps/documentos`, `apps/presentaciones` | Las **tres aplicaciones finales**, como Excel, Word y PowerPoint. Cada una empaqueta `shell` + el motor de su tipo de documento, tiene su propio ícono y se asocia a sus extensiones (`.xlsx`/`.xlsm`/`.csv`, `.docx`, `.pptx`). Un solo instalador instala las tres. Nombres comerciales por decidir, sin marcas de Microsoft | Sin lógica propia | `shell`, `edit` y el motor correspondiente | Fase 1 (hoja), 2 (documentos), 3 (presentaciones) | Por construir |
| `apps/tauri`, `apps/gpui` (`dokumentz-app-tauri`, `dokumentz-app-gpui`) | Las dos interfaces candidatas de la 001, con FR-001…FR-013 idénticos | — | `dokumentz-sheet`, `metrics`, `script` | PoC 001 | PoC 001. **Una se borra** al decidir (sección 7) |

### 4.3 Herramientas (`tools/`)

Permanentes: cada fase mide velocidad **y** compatibilidad (A4).

| Herramienta | Para qué | Estado |
|---|---|---|
| `testdata-gen` | Genera A1–A3 con semilla fija (antes `poc-gen`) | PoC 001 |
| `xlsx-diff` | Compara celda a celda con `calamine` (ida y vuelta, SC-005) | PoC 001 |
| `compat-check` | Abre cada archivo del corpus A4, recalcula y compara con el valor que el archivo trae guardado (FR-014, M-09, SC-007); nunca ejecuta macros | PoC 001 (nuevo) |
| `bench-runner`, `bench-report` | Medición externa idéntica para todas las apps y regla de decisión | PoC 001 |
| Equivalente de `compat-check` para documentos | Mide fidelidad de paginación (sección 9). Nombre por decidir en la spec 002 | Por construir (0b) |

### 4.4 Decisión: tres aplicaciones, como Office

**Decidido por el dueño (2026-10-22): tres aplicaciones separadas**, igual que Excel, Word y PowerPoint, porque Dokumentz debe sentirse como Office y **la gente no debe notar el cambio** (sección 4.5). Se descartó la app única con pestañas al estilo ONLYOFFICE Desktop.

| Aspecto | Cómo queda |
|---|---|
| Instalación | **Un solo instalador** que instala las tres apps, como Office |
| Ventanas | Una ventana por documento, como Office |
| Íconos y asociaciones | Cada app tiene su ícono y abre sus extensiones al hacer doble clic (`.xlsx`/`.xlsm`/`.csv` → hoja; `.docx` → documentos; `.pptx` → presentaciones) |
| Código compartido | Las tres usan los mismos crates (`crates/`): solo cambia `apps/`. `dokumentz-shell` es una biblioteca común |
| Portapapeles entre apps | Por el portapapeles del sistema, con varios formatos a la vez (texto, TSV, HTML, imagen) que `dokumentz-edit` escribe y lee: copiar una tabla de la hoja y pegarla en un documento funciona como entre las apps de Office |
| Ventaja | Cada app arranca solo con su motor; un fallo de una no cierra las demás; el modelo mental «Excel / Word / PowerPoint» se conserva |
| Costo | Cada binario enlaza la base común, así que el tamaño instalado se suma; se mide (M-08) y, si rebasa su presupuesto, la base se comparte como biblioteca dinámica |

### 4.5 Sentirse como Office: que la gente no note el cambio

Es un requisito de producto (constitución, principio V), no un adorno. Se separa lo que **sí** puede ser igual de lo que **no** puede o todavía no será igual.

**La vara mínima es ONLYOFFICE** (decidido por el dueño, 2026-10-22): quien lo usa no siente el cambio frente a Office, aunque no es idéntico píxel a píxel. Dokumentz debe igualar o superar ese nivel de familiaridad (estructura de la cinta, nombres, atajos, comportamientos) y ser más rápido. ONLYOFFICE se usa como referencia de experiencia y como juez lado a lado en las revisiones de paridad; **no** se copia su código ni sus íconos (principio VII).

**Igual que Office (obligatorio en cada fase):**
- Las mismas pestañas de la cinta, en el mismo orden y con los **mismos nombres que Office en español**; cada botón en el grupo donde la gente lo busca de memoria.
- Los mismos **atajos de teclado** de cada sistema (Cmd en macOS; Ctrl, F2, Alt para navegar la cinta en Windows y Linux).
- Los mismos **comportamientos**: rellenar arrastrando, autocompletar, brocha de formato, pegado especial, menú del clic derecho, Intro/Tab al editar, deshacer.
- **Funciones en el idioma del usuario** (=SUMA, =BUSCARV) y los mismos mensajes de error de celda (`#¡DIV/0!`, `#N/D`).
- **Integración con el sistema**: doble clic en el archivo abre la app correcta, «Abrir con», archivos recientes, recuperación tras un cierre inesperado, diálogo de imprimir.
- **Archivos de Office como nativos** (principio IV): guardar no convierte ni pregunta formatos.

**No puede ser idéntico (riesgo legal, principio V):**
- Íconos, colores, logotipos y el dibujo exacto de la cinta: el diseño visual es propio. La estructura (pestañas, grupos, nombres de función, atajos) sí sigue a Office. Antes de construir la cinta se revisa con asesoría legal.
- Los nombres «Excel», «Word», «PowerPoint» y «Office» no se usan.

**Lo que la gente podría notar y cómo se trata (riesgos abiertos):**

| Diferencia | Por qué | Cómo se reduce |
|---|---|---|
| **Fuentes de Microsoft** (Calibri, Cambria, Aptos y otras) | Son fuentes con licencia de Microsoft; no se pueden incluir en Dokumentz. Si faltan en el equipo, el texto se reacomoda y los documentos se ven distintos | Usar las del sistema si están instaladas (por ejemplo, por Office); si no, sustitutas libres con las mismas medidas cuando existan (Carlito para Calibri, Caladea para Cambria). Para Aptos no hay sustituta conocida verificada: `[HIPÓTESIS]`, se investiga en la PoC 002 |
| Funciones que todavía no existen | Se construye por fases | Las compuertas miden compatibilidad con archivos reales (SC-007 y equivalentes); lo no soportado se conserva al guardar (A3) y se avisa |
| Macros | Llegan en la fase 5 | Se conservan intactas desde la fase 1 y se avisa que no se ejecutan |
| Nube y coautoría en tiempo real (OneDrive, compartir enlace) | Fuera del alcance por decisión: Dokumentz es local y sin cuentas (principio I). El dueño confirmó el 2026-10-22 que **no es prioridad por ahora** | Si se quiere colaborar más adelante, hacerlo sin nube propia (*local-first*); requiere enmienda y no se planea por ahora |
| Copilot y funciones de IA en la nube | Fuera del alcance (sin red ni cuentas) | — |

Cada fase incluye en su spec una **lista de paridad con Office** (qué se comporta igual, qué no y por qué), revisada con personas que usan Office a diario.

---

## 5. Flujo de datos de una edición

```
 1 tecla / IME / ratón
        │  (el búfer de composición IME vive en la interfaz: es entrada, igual que en la 001)
        ▼
 2 dokumentz-shell    traduce la entrada a un comando según los atajos del sistema
        │             (Cmd en macOS; Ctrl y F2 en Windows y Linux)
        ▼
 3 dokumentz-edit     apply(documento, comando) → enruta al motor del documento activo
        ▼
 4 motor del documento (dokumentz-sheet | -doc | -slide)
        │             cambia el modelo (la hoja recalcula; el documento reacomoda y repagina)
        │             y devuelve el estado de interfaz + una nueva versión de datos
        ▼
 5 lista de dibujo    el motor la construye con dokumentz-text (acomodo de texto),
        │             dokumentz-drawing (formas, imágenes, gráficos) y la expresa en dokumentz-render
        ▼
 6 dokumentz-shell    decodifica la lista y dibuja (canvas 2D en Tauri / elemento propio en GPUI)
```

- La interfaz **no mide ni acomoda texto**: eso es EX-01 en la 001 y desaparece cuando `dokumentz-text` entra en la fase 1.
- Cada salto lleva un `Span` de `dokumentz-metrics`; la latencia tecla → carácter visible (M-04 en la 001) se mide igual en las tres apps.
- Mientras el área visible siga dentro del bloque ya dibujado, el desplazamiento no vuelve a llamar al núcleo (caché de bloque de la 001, plan §5.4); los motores de documentos y presentaciones hacen lo mismo por página o por diapositiva.

### Qué se comparte entre las tres apps

| Recurso | Dónde vive | Cómo lo usan |
|---|---|---|
| **Texto** | `dokumentz-text` | Celdas: recorte, desbordamiento, `####`, ajuste. Páginas: párrafos y saltos de línea. Diapositivas: cuadros de texto. Todos reciben glifos ya acomodados; la interfaz no mide |
| **Dibujos** | `dokumentz-drawing` (+ `dokumentz-render`) | El mismo DrawingML en `.xlsx`, `.docx` y `.pptx`: un gráfico, una forma o una imagen se lee, se guarda y se dibuja con el mismo código en las tres apps |
| **Portapapeles** | `dokumentz-edit` | Cada motor serializa su selección en varios formatos a la vez (texto, TSV, HTML, imagen) y sabe leerlos. La interfaz solo mueve bytes al portapapeles del sistema, sin interpretarlos. Así «hoja → documento» (tabla) y «hoja → diapositiva» usan el mismo camino que pegar entre aplicaciones |
| **Paquete de archivo** | `dokumentz-opc` | Abrir y guardar los tres formatos con las mismas reglas de conservación (A3) |
| **Medición y guiones** | `dokumentz-metrics`, `dokumentz-script` | Mismo esquema de resultados y mismos guiones para las tres apps |

---

## 6. Dónde encaja cada motor

| Motor | Papel | Qué NO es | Fase | Notas |
|---|---|---|---|---|
| **IronCalc** (0.9.1, MIT OR Apache-2.0) | **Solo** las fórmulas de la hoja y el `.xlsx` de la hoja, detrás de `dokumentz-sheet`. Ningún otro crate lo importa | No es el motor de la suite; no hace gráficos, macros, formas ni acomodo | 0a | Reevalúa todo el libro en cada edición (riesgo R-04 de la 001). Si SC-007 falla: contribuir a IronCalc usando la lista de funciones faltantes (su licencia lo permite) y volver a medir; si (b) < 90 %, evaluar alternativas de motor de fórmulas |
| **Motor de documentos** (`dokumentz-doc`) | Modelo `.docx` + paginación, sobre `dokumentz-text` | No existe aún; es lo más difícil de la suite | 0b → 2 | La PoC 002 decide si es viable y con qué candidatos (sección 9) |
| **Motor de presentaciones** (`dokumentz-slide`) | Diapositivas con posiciones explícitas, sobre la base común | — | 3 | `[HIPÓTESIS]` es más fácil que paginar documentos porque cada elemento trae su posición; se confirma con medición al llegar |
| **LibreOfficeKit** | Alternativa **evaluable** solo para documentos, como comparación en la 0b (D-05) y como plan B | No se usa para la hoja ni se adopta por defecto | 0b | API de renderizado por teselas **experimental e inestable** (`LOK_USE_UNSTABLE_API`), salida en mapa de bits BGRA de 32 bits (https://docs.libreoffice.org/libreofficekit.html, verificado 2026-10-22). Entrega imágenes, no una lista de dibujo: obligaría a la interfaz a aceptar una capa de teselas. Antes de adoptarlo se revisa contra el principio I (LGPL) y el aspecto que ya no convenció; si exige enmienda, el agente la propone y el dueño la aprueba |

---

## 7. Qué construye la PoC 001 y se queda en el producto

| Pieza de la PoC 001 | Destino |
|---|---|
| `dokumentz-sheet` (geometría, vista, navegación, edición, TSV, `open_xlsx`/`save_xlsx`, `drawlist` PDL1) | **Se queda.** Es el motor de la hoja. En la fase 1 su `drawlist` pasa a ser el primer cliente de `dokumentz-render` |
| `dokumentz-metrics`, `dokumentz-script`, `dokumentz-script-check` | **Se quedan** (transversales, compartidos por las tres apps) |
| `testdata-gen`, `xlsx-diff`, `compat-check`, `bench-runner`, `bench-report` y `criteria.toml` | **Se quedan** como herramientas permanentes |
| `tests/functional/` (guiones JSON), `tests/fixtures/`, CI de 3 SO, `deny.toml` | **Se quedan** |
| Interfaz **ganadora** de la regla de decisión (`apps/tauri` o `apps/gpui`) | **Se queda** y es la semilla de `dokumentz-shell` y de `apps/hoja` |
| Interfaz **perdedora** | **Se descarta** (se borra del árbol; el historial de git la conserva y `decision.md` registra por qué). **Es lo único que se descarta** |
| Excepción EX-01 | Caduca al cierre de la fase 0: se cierra cuando `dokumentz-text` entra en la fase 1 |

Advertencia honesta: la interfaz ganadora de la 001 no tiene cinta, ni pantalla de inicio, ni más tipos que la hoja. Es una semilla, no el producto: la fase 1 la reorganiza en `dokumentz-shell`.

---

## 8. Hoja de ruta nueva

Reemplaza a «0 PoC → 1 hoja básica → 2 hoja día a día → 3 textos → 4 presentaciones → 5 macros».

| Fase | Contenido | Compuerta de salida |
|---|---|---|
| **0a — PoC 001 hoja** | Lo ya especificado + SC-007. Construye `dokumentz-sheet`, `dokumentz-metrics`, `dokumentz-script` y las herramientas como piezas de producto | Regla de decisión de interfaz (spec 001 §9) **y** SC-007 del motor: ≥ 95 % de A4 abre sin error y 0 cierres; ≥ 98 % de fórmulas coinciden con Excel en archivos hechos por Excel; 100 % de valores sin fórmula idénticos |
| **0b — PoC 002 documentos** | Mostrar (solo lectura) páginas de `.docx` reales con un motor de texto en Rust; medir fidelidad (saltos de línea, de página, número de páginas) y velocidad frente a ONLYOFFICE y LibreOffice como jueces (y Word si está disponible). Puede empezar cuando la 001 pase su CP-1 | ¿Es viable el motor de documentos? Si no, plan B (LibreOfficeKit para documentos u otro) **antes de la fase 2** |
| **1 — Base común + hoja básica** | `dokumentz-opc`, `-text`, `-render`, `-edit`, `-shell`; cinta estilo Office (con revisión legal previa); editar formato; ordenar y filtrar; conservar partes no soportadas | Velocidad y compatibilidad medidas |
| **2 — Documentos básicos** | Abrir, editar y guardar `.docx` sobre la misma base: estilos, tablas, imágenes, encabezados, listas | Ídem |
| **3 — Presentaciones básicas** | Diapositivas, texto, imágenes, formas, modo presentador, sobre la misma base | Ídem |
| **4 — Día a día en las tres** | Gráficos comunes, tablas dinámicas, control de cambios y comentarios, patrones y animaciones, imprimir y PDF comunes | Ídem |
| **5 — Macros** | Subconjunto común de VBA o conversión | Ídem |

Reglas de las compuertas:

- «Velocidad y compatibilidad medidas» significa: JSON de `dokumentz-bench/1` con presupuestos del principio II **y** informe de compatibilidad sobre un corpus de archivos reales de esa fase, informado por fuente (usuario / Apache POI / LibreOffice; los del usuario pesan más). Los umbrales numéricos se escriben en la spec de cada fase **antes** de medir (principio VIII); no se inventan aquí.
- Las mediciones en Windows y Linux siguen pendientes de hardware (solo hay Mac); la decisión de interfaz es provisional para ellos (principio VI).
- Los archivos de internet no son confiables: nunca se ejecutan macros ni se abren en Excel con macros habilitadas. La descarga del corpus A4 requiere el visto bueno del dueño (T114).
- Orden: la 0b arranca en paralelo con el resto de la 001 una vez pasado su CP-1 (núcleo `dokumentz-sheet` completo y probado); la fase 1 no empieza hasta cerrar 0a, 0b y la decisión de la sección 11.

---

## 9. Esbozo de la PoC 002 — documentos

> La spec completa (`specs/002-poc-documentos/`: `spec.md`, `research.md`, `plan.md`, `tasks.md`) **se escribe después, cuando la 001 pase CP-1**. Esto es solo el esbozo para que la arquitectura tenga el riesgo más grande identificado desde ahora.

### 9.1 Preguntas que debe responder

1. ¿Un motor de texto en Rust (shaping + saltos de línea + bidi) reproduce los **saltos de línea** de Word, ONLYOFFICE y LibreOffice en párrafos de `.docx` reales?
2. ¿La **paginación** coincide: número de páginas y dónde cae cada salto de página (tablas, encabezados y pies, imágenes, columnas, saltos de sección, viudas y huérfanas)?
3. ¿Qué tan rápido abre y se desplaza (tiempo a la primera página, cambio de página, memoria) frente a ONLYOFFICE y LibreOffice?
4. ¿Qué pasa con las **fuentes** (Calibri, Cambria, Times New Roman y similares no están en todos los sistemas)? Cuando falta la fuente original, el salto de línea cambia. Se evalúan fuentes de licencia libre y métricas compatibles (su licencia se verifica en la spec 002).
5. ¿El mismo `dokumentz-text` sirve a la vez a celdas (muchas y rápidas) y a páginas (pocas y fieles)? Si no, habrá dos motores de texto.
6. ¿Se adopta `rdocx` entero, solo su modelo (`rdocx-oxml`), o se escribe el modelo propio sobre `dokumentz-opc`?
7. ¿Cómo se compara con LibreOfficeKit en fidelidad, velocidad y aspecto (D-05)?

### 9.2 Candidatos (verificados el 2026-10-22)

| Candidato | Versión | Licencia | Última publicación | Para qué sirve | Observaciones |
|---|---|---|---|---|---|
| `rdocx` | 0.16.3 | MIT OR Apache-2.0 | 2026-10-22 (creado 2026-02-22; 47 198 descargas) | Leer, escribir y convertir DOCX; incluye `rdocx-layout` («Word flow layout») y salida PDF/PNG/SVG | **Muy joven** (8 meses; 44 estrellas). La página no nombra biblioteca de shaping; el `Cargo.toml` de `rdocx-layout` solo muestra `ttf-parser` y crates `oxml-*`, sin shaper visible: **a verificar en la 0b**. Dice conservar el XML que no modela (sin declarar cobertura completa). No ejecuta VBA |
| `parley` | 0.12.0 | Apache-2.0 OR MIT | 2026-10-09 (4,56 M de descargas) | Acomodo de texto enriquecido: posiciones de glifos, saltos de línea, bidi (ICU4X), selección y edición | Usa HarfRust para shaping, `fontique` para fuentes y `skrifa` para contornos. MSRV 1.88. Linebender (https://github.com/linebender/parley) |
| `cosmic-text` | 0.19.0 | MIT OR Apache-2.0 | 2026-04-22 (10,8 M de descargas) | Texto multilínea en Rust puro: acomodo propio, bidi, respaldo de fuentes | Shaping con HarfRust. Su README presenta una hoja de ruta con funciones pendientes (no se declara terminado). Modos de ajuste de línea listados: simple, con sangría, sin ajuste y elipsis |
| `swash` | 0.2.10 | Apache-2.0 OR MIT | 2026-07-17 (16,4 M de descargas) | Introspección de fuentes, shaping complejo y render de glifos | Es una pieza baja, no un acomodo de párrafos |
| `rustybuzz` | 0.20.1 | MIT | 2024-11-12 | Port de HarfBuzz | **Descartado para trabajo nuevo**: su repositorio fue archivado el 2026-07-26 («unmaintained, and archived») y recomienda **HarfRust** |
| `harfrust` (sucesor) | 0.15.0 | MIT | 2026-10-22 | Port de HarfBuzz (shaping) | Lo usan `parley` y `cosmic-text`. Se evalúa en lugar de `rustybuzz` |

Fuentes (todas consultadas el 2026-10-22): https://crates.io/crates/rdocx · https://github.com/tensorbee/rdocx · https://raw.githubusercontent.com/tensorbee/rdocx/main/crates/rdocx-layout/Cargo.toml · https://crates.io/crates/parley · https://github.com/linebender/parley · https://crates.io/crates/cosmic-text · https://github.com/pop-os/cosmic-text · https://crates.io/crates/swash · https://crates.io/crates/rustybuzz · https://github.com/harfbuzz/rustybuzz · https://crates.io/crates/harfrust.

Capas posibles (a confirmar): fuentes y shaping (`harfrust`, `swash`) → acomodo de párrafo (`parley` o `cosmic-text`) → paginación de documento (propia o `rdocx-layout`) → `dokumentz-render`.

### 9.3 Cómo medir la fidelidad

- **Corpus** `.docx` con el mismo criterio que A4: archivos del usuario, de Apache POI y de LibreOffice, **informado por fuente** (los de POI y LibreOffice incluyen muchos casos raros o de reportes de errores). Descarga con visto bueno del dueño; sin ejecutar macros.
- **Jueces:** PDF exportado por ONLYOFFICE (`x2t` se usa como programa externo en la 001; que convierta `.docx` a PDF en el equipo se verifica en la 0b), por LibreOffice en modo sin ventana y, si está disponible, por Word. Se lee el PDF con una herramienta externa de texto con posiciones (se elige en la spec 002; se invoca como programa, igual que `x2t`).
- **Medidas** por documento: (a) ¿mismo **número de páginas**?; (b) ¿la primera línea de cada párrafo cae en la **misma página**?; (c) ¿mismo **número de líneas por párrafo** y mismas palabras al inicio de línea?; (d) velocidad: tiempo a la primera página, cambio de página (p95), memoria.
- **Acuerdo entre jueces:** ONLYOFFICE y LibreOffice discrepan entre sí. La regla compara a Dokumentz contra cada juez **y** contra el acuerdo entre los jueces: no se le exige parecerse más a un juez de lo que los jueces se parecen entre sí. Word, si está, manda.
- **Reglas antes de medir** (principio VIII): umbrales, método de 5 repeticiones, mediana y p95 se fijan en la spec 002 y se congelan con etiqueta git. Se reportan también las medidas donde se pierde.
- **Estado de la fidelidad `[HIPÓTESIS]`:** el estándar describe el formato del archivo, no el algoritmo exacto de paginación de Word; no se verificó esto en esta sesión. Por eso la 0b mide en lugar de prometer.

### 9.4 Plan B

Si la 0b dice «no viable» (o viable solo en documentos simples), antes de la fase 2:

1. **LibreOfficeKit para documentos** (comparación ya prevista en D-05): visor de teselas; se revisa el principio I (LGPL) y el aspecto.
2. **Adoptar o bifurcar el acomodo de `rdocx`** y contribuir lo que falte, si las licencias lo permiten.
3. **Reducir el alcance**: fidelidad alta en documentos simples; los complejos se abren con aviso y se conservan intactos al guardar (A3).

El plan B no cambia las capas de arriba: `dokumentz-doc` se reemplaza detrás de `dokumentz-edit` (A6).

---

## 10. Riesgos de la suite completa

| # | Riesgo | Mitigación | Señal para replantear |
|---|---|---|---|
| S1 | **Alcance enorme**: reemplazar tres programas de Office, con macros | Fases con compuerta de salida; cada fase deja un producto útil (tras la fase 1 ya hay una hoja usable). Se puede parar sin perder lo hecho | Una fase tarda el doble de lo previsto y su compuerta no se acerca |
| S2 | **Pocas personas** (el dueño y agentes; sin fines de lucro): el mantenimiento de motores y compatibilidad no escala | Base común para no repetir trabajo; etapas cortas con agentes (principio IX); contribuir a proyectos de los que depende | La compatibilidad (SC-007 y equivalentes) retrocede entre fases |
| S3 | **Fidelidad de Word**: paginación sin algoritmo público, fuentes ausentes, modos de compatibilidad | Se prueba primero en la 0b, con jueces y reglas fijadas antes (sección 9) | La 0b y el plan B quedan por debajo del umbral en documentos de uso diario |
| S4 | **Motores jóvenes**: IronCalc 0.9.1 (pre-1.0), `rdocx` 0.16 (8 meses), `parley` 0.12, `harfrust` 0.15, GPUI sin versión estable. `rustybuzz` ya fue archivado: el abandono ocurre | Versiones exactas fijas; capa propia delgada sobre cada motor para poder cambiarlo; `cargo deny`; contribuir upstream | Un motor se archiva o rompe la API sin sucesor |
| S5 | **IronCalc reevalúa todo el libro** en cada edición | Medido en la 001 (M-04, M-05 en A2; riesgo R-04) | Latencia de edición fuera de presupuesto en libros grandes y sin remedio upstream |
| S6 | **Compatibilidad de fórmulas** no medida hasta ahora | SC-007 con `compat-check`; si falla, plan de motor antes de la fase 1 | (b) < 90 %: evaluar alternativas de motor de fórmulas |
| S7 | **El texto de la interfaz** `[HIPÓTESIS]`: si gana Tauri, el canvas 2D dibuja cadenas, no identificadores de glifo; un acomodo hecho por `dokumentz-text` podría no coincidir con el texto de la webview | `dokumentz-render` debe poder expresar texto como cadena + fuente + posición y/o contornos de glifo; la fuente se carga en la webview desde el mismo archivo. Se prueba en la fase 1 con la interfaz ganadora (no se probó en esta sesión) | El texto de la webview no coincide con el acomodo calculado sin una solución barata |
| S8 | **Macros (fase 5)**: VBA es un lenguaje y un entorno enormes | Conservar `vbaProject.bin` desde la fase 1 (A3); la fase 5 decide subconjunto o conversión con datos de uso real | Los archivos reales dependen de macros que el subconjunto no cubre |
| S9 | **Cinta y diseño visual** (principio V): riesgo legal | Revisión con asesoría legal antes de construirla (fase 1); diseño visual propio | Dictamen legal adverso |
| S10 | **Solo hay Mac**: Windows y Linux sin medir (principio VI) | Decisión provisional, CI funcional de tres sistemas, medición cuando haya hardware | Resultados en Windows o Linux contradicen la decisión de interfaz |

### Señales que harían replantear la arquitectura

- La 0b y su plan B fallan: la suite pasaría a «hoja + visor de documentos y presentaciones».
- `dokumentz-text` no puede servir a celdas y a páginas a la vez: dos motores de texto, y se revisa la base común.
- El tamaño instalado de las tres apps (cada una enlaza la base común) rebasa su presupuesto: la base se comparte como biblioteca dinámica.
- El protocolo de dibujo se vuelve el cuello de botella en la interfaz ganadora (copias por el puente en Tauri): se replantea el formato.
- SC-007 (b) < 90 % y ninguna alternativa de motor de fórmulas mejora: se replantea el motor de la hoja.

---

## 11. Decisiones pendientes

Ninguna. La última (app única o tres apps) se resolvió el 2026-10-22: **tres apps, como Office** (sección 4.4). Quedan por decidir más adelante, sin bloquear nada ahora: los nombres comerciales de las tres apps (sin marcas de Microsoft) y la revisión legal de la cinta (antes de la fase 1).

Todo este documento está decidido (2026-10-22). Cualquier cambio a una decisión cerrada se trata como enmienda propuesta por escrito, con datos (Gobierno, regla 3).
