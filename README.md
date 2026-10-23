# Dokumentz

Suite de oficina libre escrita en Rust: hoja de cálculo, documentos y presentaciones,
con formatos de Office como formato nativo.

El proyecto está en la **fase 0** (pruebas de concepto): hoy solo existe el esqueleto del
repositorio y el motor de la hoja de cálculo se está probando contra dos interfaces
candidatas (Tauri y GPUI). Nada de esto está listo para usarse todavía.

## Documentación

Las decisiones, la arquitectura y los planes viven en [`specs/`](specs/):

- [`specs/constitucion.md`](specs/constitucion.md): principios del proyecto.
- [`specs/000-arquitectura-suite/`](specs/000-arquitectura-suite/): arquitectura de la suite.
- [`specs/001-poc-hoja-de-calculo/`](specs/001-poc-hoja-de-calculo/): PoC de la hoja de cálculo
  (especificación, plan y tareas).

## Compilar y probar

El toolchain (Rust 1.99.0) se fija en `rust-toolchain.toml`.

```sh
cargo test-core                 # pruebas del núcleo y las herramientas (sin interfaces)
cargo build --workspace --release
cargo deny check licenses bans sources
```

## Licencia

Doble licencia, a elección de quien la use: [MIT](LICENSE-MIT) o
[Apache-2.0](LICENSE-APACHE) (`MIT OR Apache-2.0`).
