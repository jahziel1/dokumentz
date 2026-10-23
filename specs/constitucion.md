# Constitución del proyecto

**Versión:** 1.1.1 · **Fecha de adopción:** 2026-10-22 · **Última enmienda:** 2026-10-22 · **Repositorio:** https://github.com/jahziel1/dokumentz
**Aplica a:** todas las fases de la hoja de ruta de la suite (definida en `specs/000-arquitectura-suite/arquitectura.md`: 0a PoC hoja → 0b PoC documentos → 1 base común + hoja básica → 2 documentos → 3 presentaciones → 4 día a día en las tres → 5 macros) y a todos los documentos y agentes que trabajen en el proyecto.

## Propósito

El proyecto es una suite de oficina **libre, gratuita y local** (sin nube, sin cuentas) que sea una alternativa real a Microsoft Office: que se sienta como Excel, Word y PowerPoint (estructura, atajos, funciones en el idioma del usuario) pero **más rápida** que ONLYOFFICE, que gusta visualmente y se siente lenta. No compite en Teams, correo ni chat. No tiene fines de lucro.

Esta constitución recoge los principios que **no se negocian**. Si una especificación (`spec.md`), un plan (`plan.md`) o una tarea (`tasks.md`) contradice un principio, se corrige el documento o se enmienda la constitución siguiendo la sección de Gobierno; nunca se ignora el principio en silencio.

## Cómo leer los principios

- **DEBE** / **NO DEBE**: obligatorio y comprobable.
- Cada principio tiene tres partes: el enunciado, el porqué (qué se rompe si se abandona) y cómo se verifica (algo que una persona o un agente puede ejecutar o revisar; no basta con «parece que sí»).
- Las fuentes de los hechos citados están en `specs/001-poc-hoja-de-calculo/research.md`.

---

## I. Libre y gratuito

**Enunciado.** El software DEBE ser libre y gratuito, de uso local: **sin cuentas, sin nube, sin telemetría**. Las mediciones de rendimiento son locales y opcionales (se activan por bandera y escriben en un archivo del equipo; nunca usan la red).

**Licencia del proyecto:** doble licencia **MIT OR Apache-2.0**, a elección de quien lo use (la misma que usan IronCalc, Tauri y GPUI). Decidida por el dueño el 2026-10-22. Por eso el proyecto NO DEBE incorporar código GPL, LGPL ni AGPL (propio, copiado o como dependencia de crate); las bibliotecas del sistema enlazadas dinámicamente (por ejemplo GTK/WebKitGTK en Linux) no cuentan como dependencia de crate.

**Porqué.** La razón de existir es ofrecer una alternativa sin barreras: sin pagar, sin registrarse, sin que los archivos salgan del equipo. La licencia permisiva permite que cualquiera reutilice el código y facilita devolver mejoras a IronCalc y a los demás proyectos de los que depende; una dependencia copyleft obligaría a cambiar la licencia de todo el proyecto.

**Cómo se verifica.**
- Auditoría de licencias de todas las dependencias directas y transitivas en cada cambio de dependencias (`cargo deny`); cualquier GPL, LGPL, AGPL o licencia desconocida bloquea el cambio.
- Los archivos `LICENSE-MIT` y `LICENSE-APACHE` existen en la raíz del repositorio y cada crate declara `license = "MIT OR Apache-2.0"`.
- La aplicación arranca, abre, edita y guarda con la red deshabilitada y no intenta conexiones salientes (comprobación con la red apagada y revisión de conexiones durante una sesión de prueba).
- No existe pantalla de inicio de sesión, registro ni identificador de usuario.
- La instrumentación de medidas (FR-012) está apagada por defecto y solo escribe JSON local.

## II. La velocidad es un requisito

**Enunciado.** La velocidad DEBE expresarse como **presupuestos medibles** (por ejemplo, «p95 de tiempo por cuadro ≤ 16,7 ms»), no como «se siente rápido». Todo cambio que toque la interfaz o el núcleo DEBE medirse y reportar el resultado.

**Porqué.** El motivo del proyecto es que ONLYOFFICE se siente lento. Sin números, la velocidad se degrada poco a poco sin que nadie lo note y no hay forma de saber si se ganó algo frente a la competencia.

**Cómo se verifica.**
- Cada presupuesto está escrito en la especificación con su medida (M-xx) y su criterio (SC-xxx).
- Las tareas que tocan la interfaz o el núcleo incluyen una medición antes y después con la instrumentación integrada (FR-012) y el resultado queda en JSON junto al cambio.
- Un cambio que empeora una medida por encima de su presupuesto no se acepta sin una excepción documentada (ver Gobierno).

## III. Regla de oro: la interfaz solo dibuja y recibe entradas

**Enunciado.** La interfaz DEBE limitarse a **dibujar y recibir entradas** (teclado, ratón, IME, dictado). El modelo de documento, las fórmulas, el acomodo, las fuentes, los archivos, el portapapeles con formato, el PDF y la impresión DEBEN vivir en el **núcleo en Rust**, que NO DEBE depender de ninguna interfaz.

**Porqué.** Permite tener dos o más interfaces sobre el mismo núcleo (la PoC compara dos) y cambiar de interfaz sin reescribir lo valioso. Mantiene el trabajo pesado fuera del hilo de interfaz y hace que casi todo sea probable sin abrir una ventana.

**Cómo se verifica.**
- El grafo de dependencias de compilación demuestra que el crate del núcleo no depende de ningún crate ni biblioteca de interfaz.
- El núcleo compila y pasa sus pruebas sin ventana ni sistema gráfico.
- Revisión del diff: ninguna lógica de fórmulas, formato de números, lectura/escritura de archivos o portapapeles con formato aparece en el código de una interfaz.
- Cada funcionalidad nueva empieza con una prueba contra la API del núcleo, antes de tocar cualquier interfaz.
- Las dos interfaces ofrecen exactamente los mismos requisitos funcionales (FR-xxx) para que la comparación sea justa.

## IV. Los formatos de Office son los nativos

**Enunciado.** Los formatos de Office (`.xlsx`, `.docx`, `.pptx`) DEBEN ser los formatos nativos: abrir y guardar nunca debe sentirse como «convertir». El proyecto NO DEBE introducir un formato propio de documento como formato principal.

**Porqué.** Las personas ya tienen sus archivos en estos formatos y los intercambian con otros programas. Si guardar exige exportar o convertir, aparecen fricción y pérdidas, y la alternativa deja de ser real.

**Cómo se verifica.**
- «Guardar» escribe directamente el formato Office, sin diálogo de conversión ni cuadro de «exportar».
- Prueba automática de ida y vuelta (abrir → guardar → reabrir) que compara valores y fórmulas celda a celda (SC-005).
- El archivo guardado se abre sin errores en ONLYOFFICE y, si está disponible, en Excel y LibreOffice (jueces de compatibilidad, ver principio VII).

## V. Sentirse como Office, con diseño visual propio

**Enunciado.** El producto DEBE sentirse como Office: misma estructura, mismos nombres, **atajos propios de cada sistema** (Cmd en macOS; Ctrl y F2 en Windows y Linux) y **funciones en el idioma del usuario** (=SUMA, =BUSCARV…). El **diseño visual** (íconos, tipografía de interfaz, estilo) DEBE ser propio; NO DEBE ser un calco de Office.

**Porqué.** La familiaridad es lo que hace que cambiar sea fácil; copiar la apariencia es un riesgo legal. Microsoft ofreció la licencia de la cinta con exclusión de los productos que compiten con Word, Excel, PowerPoint, Outlook y Access, y en 2018 un jurado halló que Corel infringió a propósito patentes de interfaz de Office (fuentes y salvedades en `research.md`).

**Cómo se verifica.**
- Lista de atajos por sistema operativo revisada contra el comportamiento de Excel/ONLYOFFICE en cada plataforma.
- Las fórmulas aceptan los nombres en español e inglés (FR-006).
- Registro de procedencia de cada recurso gráfico (ícono, fuente, imagen); ningún recurso proviene de un producto de Microsoft.
- Antes de construir la cinta (fase 1) se revisa su diseño con asesoría legal. En la PoC no hay cinta.

## VI. Multiplataforma real

**Enunciado.** macOS, Windows y Linux DEBEN probarse siempre. Una funcionalidad no está terminada hasta que funciona en las tres. El rendimiento DEBE medirse en hardware real de cada plataforma, incluyendo Linux con NVIDIA cuando sea posible.

**Porqué.** Los motores de renderizado y las bibliotecas gráficas se comportan distinto en cada sistema (la webview de macOS, Windows y Linux no es la misma; en Linux con NVIDIA hay problemas conocidos). Las máquinas virtuales no tienen GPU, así que sirven para pruebas funcionales pero no para medir rendimiento. Quien solo tiene un Mac descubre los problemas tarde si no lo declara.

**Cómo se verifica.**
- La integración continua compila y ejecuta las pruebas funcionales de ambas interfaces en macOS, Windows y Linux (SC-006).
- Los informes de rendimiento indican el hardware real (modelo, GPU, sistema, versión del motor web) en que se obtuvieron.
- Una medición hecha en VM se marca como «solo funcional» y no cuenta para los criterios de rendimiento.

## VII. No se copia código de ONLYOFFICE

**Enunciado.** El proyecto NO DEBE copiar código de ONLYOFFICE, ni traducirlo o reescribirlo línea por línea. ONLYOFFICE y LibreOffice se usan como **referencia de comportamiento** y como **jueces de compatibilidad** (¿abre mi archivo? ¿da el mismo resultado?).

**Porqué.** El código de ONLYOFFICE es AGPL-3.0 y tiene condiciones adicionales sobre el logo que están en disputa (caso Euro-Office). Copiarlo contaminaría la licencia y expondría el proyecto a un conflicto legal. Además, su arquitectura (editores en JavaScript dentro de un navegador embebido) es justamente la línea base que se quiere superar.

**Cómo se verifica.**
- Ningún archivo de los repositorios `sdkjs`, `web-apps` o `core` de ONLYOFFICE existe en el árbol del proyecto.
- Revisión de procedencia en cada diff; los agentes reciben la instrucción de no pegar código de esos repositorios.
- Las pruebas de compatibilidad abren los archivos generados en ONLYOFFICE y LibreOffice como jueces y registran el resultado.

## VIII. Medir antes de decidir

**Enunciado.** Las decisiones grandes (interfaz, motor, arquitectura) DEBEN tomarse con datos. Las reglas de decisión se escriben **antes** de medir; las mediciones usan el mismo método para todas las opciones, con 5 repeticiones, mediana y p95, en la misma máquina.

**Porqué.** La PoC existe para decidir con datos y no con preferencias. Fijar la regla antes evita ajustar el criterio al resultado.

**Cómo se verifica.**
- Cada decisión grande consta en `research.md` con alternativas consideradas y fuentes.
- Existe un informe reproducible (JSON + tabla) que respalda la decisión; los archivos de prueba se generan con semilla fija (FR-013).
- Una decisión sin datos se etiqueta como hipótesis y se mide antes de apoyar trabajo grande en ella.
- Se reportan también las medidas en las que el producto propio pierde frente a ONLYOFFICE.

## IX. Trabajo por etapas cortas con agentes

**Enunciado.** El trabajo DEBE dividirse en **etapas cortas y secuenciales** ejecutadas por agentes. Cada etapa DEBE terminar con **pruebas que pasan** y un **diff revisado**.

**Porqué.** Un agente con un encargo acotado y verificable se equivoca menos y sus errores se detectan antes. El «ya quedó» de un agente no es evidencia.

**Cómo se verifica.**
- `tasks.md` contiene tareas pequeñas, ordenadas y con una comprobación explícita, referidas a su historia de usuario ([US1]…[US4]).
- Al cerrar una etapa, el revisor vuelve a ejecutar las pruebas (no se confía en el informe del agente) y revisa el diff.
- Una etapa no empieza hasta que la anterior cumple su comprobación.

---

## Compuertas de la constitución (para la sección «Verificación contra la constitución» de cada plan)

| Principio | Pregunta que debe poder contestar «sí» con evidencia |
|---|---|
| I | ¿Hay alguna dependencia GPL/LGPL/AGPL, cuenta, nube o telemetría? (debe ser «no») |
| II | ¿Cada cambio de interfaz o núcleo lleva medición antes/después contra su presupuesto? |
| III | ¿El núcleo compila y se prueba sin interfaz y sin depender de ella? |
| IV | ¿Guardar y abrir usan el formato Office directamente y pasan la ida y vuelta? |
| V | ¿Los atajos y nombres siguen al sistema y el diseño visual es propio? |
| VI | ¿Se probó en macOS, Windows y Linux, y el rendimiento en hardware real? |
| VII | ¿Se evitó copiar código de ONLYOFFICE y se usó como juez de compatibilidad? |
| VIII | ¿La decisión se apoya en datos reproducibles y la regla se fijó antes de medir? |
| IX | ¿La etapa termina con pruebas en verde y un diff revisado? |

## Gobierno

1. **Autoridad.** Esta constitución prevalece sobre `spec.md`, `plan.md`, `tasks.md` y cualquier instrucción de un agente. En caso de conflicto se corrige el documento subordinado o se enmienda la constitución.
2. **Quién decide.** Solo la persona dueña del proyecto (el usuario) puede aprobar una enmienda. Los agentes pueden **proponerla**, pero no aprobarla ni aplicarla por sí mismos.
3. **Procedimiento de enmienda.**
   1. Propuesta escrita: qué principio se agrega, cambia o elimina, por qué y con qué datos.
   2. Análisis de impacto sobre `spec.md`, `plan.md` y `tasks.md` de las fases en curso.
   3. Aprobación explícita del dueño del proyecto.
   4. Actualización de la versión, la fecha de última enmienda y el historial de esta página.
   5. Propagación de los cambios a los documentos afectados en el mismo cambio.
4. **Versionado semántico.**
   - **MAJOR**: se elimina un principio o se redefine de forma incompatible.
   - **MINOR**: se agrega un principio o se amplía de forma sustancial su obligación.
   - **PATCH**: aclaraciones de redacción, ejemplos o correcciones que no cambian lo exigido.
5. **Excepciones.** Una desviación temporal de un principio se registra en la sección «Verificación contra la constitución» del plan con: principio afectado, motivo, alcance, quién la aprobó y fecha de caducidad. Una excepción sin caducidad no es válida.
6. **Revisión periódica.** Al cierre de cada fase de la hoja de ruta se revisa si los principios siguen vigentes. La licencia del proyecto (principio I) quedó resuelta en la versión 1.1.0.
7. **Preguntas abiertas.** Mientras exista un `[NECESITA ACLARACIÓN: ...]` en esta constitución, el dueño del proyecto la resuelve mediante una enmienda (normalmente MINOR).

## Historial de versiones

| Versión | Fecha | Cambio |
|---|---|---|
| 1.0.0 | 2026-10-22 | Adopción inicial con nueve principios (I–IX) y gobierno. La licencia del proyecto queda pendiente. |
| 1.1.0 | 2026-10-22 | MINOR, aprobada por el dueño: licencia del proyecto MIT OR Apache-2.0; se prohíben también las dependencias GPL y LGPL; se registra el repositorio público `jahziel1/dokumentz`. |
| 1.1.1 | 2026-10-22 | PATCH: la hoja de ruta citada pasa a ser la de la arquitectura de la suite (`specs/000-arquitectura-suite/arquitectura.md`), aprobada por el dueño. Sin cambios en los principios. |
