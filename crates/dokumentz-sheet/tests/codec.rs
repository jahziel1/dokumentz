//! T018: formato binario v1 de la lista de dibujo.
//!
//! El archivo `tests/golden/drawlist_v1.bin` congela el formato: es el contrato con
//! `drawlist.ts`. Se genera desde `dibujo_dorado()`, un `DrawList` armado a mano (no desde el
//! motor), para que no cambie si cambia IronCalc. Para regenerarlo a propósito:
//!
//! ```text
//! cargo test -p dokumentz-sheet --test codec regenerar_dorado -- --ignored
//! ```

mod common;

use dokumentz_sheet::codec::{
    CELL_LEN, HEADER_LEN, MAGIC, STYLE_LEN, VERSION, decode, encode, encoded_len,
};
use dokumentz_sheet::drawlist::MAX_SPILL;
use dokumentz_sheet::geometry::{MAX_COLS, MAX_ROWS};
use dokumentz_sheet::{
    Align, CellStyle, CodecError, Core, DrawCell, DrawList, DrawRequest, Kind, Rgba, StrRef, VAlign,
};

use common::{abrir, crear_small};

const RUTA_DORADO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/drawlist_v1.bin");

/// Bytes del archivo dorado versionado. Se lee en tiempo de ejecución (y no con
/// `include_bytes!`) para poder regenerarlo cuando todavía no existe.
fn dorado() -> Vec<u8> {
    std::fs::read(RUTA_DORADO).expect("falta tests/golden/drawlist_v1.bin")
}

/// Añade una cadena al búfer y devuelve su referencia.
fn cadena(buf: &mut String, s: &str) -> StrRef {
    let off = buf.len() as u32;
    buf.push_str(s);
    StrRef {
        off,
        len: s.len() as u32,
    }
}

/// Lista determinista que ejercita todo el formato: 3 filas x 4 columnas (la tercera columna
/// oculta, de ancho 0) a partir de la fila 1000 y la columna 27 («AA») de la hoja 2, 4 estilos,
/// 7 celdas de todos los tipos, texto con acentos, CJK y emoji, una cadena vacía y un relleno.
fn dibujo_dorado() -> DrawList {
    let mut s = String::new();
    let col_labels = ["AA", "AB", "AC", "AD"].map(|t| cadena(&mut s, t)).to_vec();
    let row_labels = ["1000", "1001", "1002"].map(|t| cadena(&mut s, t)).to_vec();
    let t0 = cadena(&mut s, "Texto ñ — 日本語 🚀");
    let t1 = cadena(&mut s, "1,234.50");
    let t2 = cadena(&mut s, "TRUE");
    let t3 = cadena(&mut s, "#DIV/0!");
    let t4 = StrRef::default(); // celda de solo estilo
    let t5 = cadena(&mut s, "");
    let t6 = cadena(&mut s, "fin");
    let estilos = vec![
        CellStyle {
            fg: Rgba::NEGRO,
            fill: None,
            font_px: 17.1875,
            family: 0,
            bold: false,
            italic: false,
            underline: false,
            strike: false,
            wrap: false,
            h_align: Align::Izq,
            v_align: VAlign::Abajo,
        },
        CellStyle {
            bold: true,
            h_align: Align::Der,
            ..CellStyle::default()
        },
        CellStyle {
            fg: Rgba::rgb(0x33, 0x66, 0x99),
            fill: Some(Rgba::rgb(0xFF, 0xFF, 0x00)),
            font_px: 21.875,
            family: 2,
            bold: true,
            italic: true,
            underline: true,
            strike: true,
            wrap: true,
            h_align: Align::Centro,
            v_align: VAlign::Arriba,
        },
        CellStyle {
            fg: Rgba::rgb(0xFF, 0x00, 0x00),
            fill: None,
            font_px: 30.0,
            family: 3,
            v_align: VAlign::Centro,
            ..CellStyle::default()
        },
    ];
    let celda = |r, c, text, style, kind, spill| DrawCell {
        row_off: r,
        col_off: c,
        text,
        style,
        kind,
        spill_cols: spill,
    };
    DrawList {
        sheet: 2,
        data_version: 0x0102_0304_0506_0708,
        first_row: 1000,
        first_col: 27,
        n_rows: 3,
        n_cols: 4,
        origin: (2340.0, 24_975.0),
        col_edges: vec![0.0, 90.0, 270.5, 270.5, 360.5],
        row_edges: vec![0.0, 25.0, 71.875, 96.875],
        col_labels,
        row_labels,
        styles: estilos,
        cells: vec![
            celda(0, 0, t0, 0, Kind::Texto, 3),
            celda(0, 1, t1, 1, Kind::Numero, 0),
            celda(0, 3, t2, 3, Kind::Bool, 0),
            celda(1, 0, t3, 0, Kind::Error, 0),
            celda(1, 2, t4, 2, Kind::SoloEstilo, 0),
            celda(2, 1, t5, 0, Kind::Texto, 0),
            celda(2, 3, t6, 3, Kind::Texto, MAX_SPILL),
        ],
        strings: s,
    }
}

/// Un bloque vacío de `n_rows` x `n_cols` con tamaños por defecto y etiquetas vacías.
fn bloque_vacio(first_row: i32, first_col: i32, n_rows: u32, n_cols: u32) -> DrawList {
    DrawList {
        sheet: 0,
        data_version: 1,
        first_row,
        first_col,
        n_rows,
        n_cols,
        origin: (0.0, 0.0),
        col_edges: (0..=n_cols).map(|i| i as f32 * 90.0).collect(),
        row_edges: (0..=n_rows).map(|i| i as f32 * 25.0).collect(),
        col_labels: vec![StrRef::default(); n_cols as usize],
        row_labels: vec![StrRef::default(); n_rows as usize],
        styles: vec![],
        cells: vec![],
        strings: String::new(),
    }
}

fn parchear(bytes: &[u8], desplazamiento: usize, nuevo: &[u8]) -> Vec<u8> {
    let mut v = bytes.to_vec();
    v[desplazamiento..desplazamiento + nuevo.len()].copy_from_slice(nuevo);
    v
}

/// Desplazamientos de cada sección del dorado (4 columnas, 3 filas, 4 estilos, 7 celdas).
struct Secciones {
    aristas_cols: usize,
    aristas_filas: usize,
    etiquetas: usize,
    estilos: usize,
    celdas: usize,
    cadenas: usize,
}

fn secciones() -> Secciones {
    let aristas_cols = HEADER_LEN;
    let aristas_filas = aristas_cols + 4 * 5;
    let etiquetas = aristas_filas + 4 * 4;
    let estilos = etiquetas + 8 * (4 + 3);
    let celdas = estilos + STYLE_LEN * 4;
    let cadenas = celdas + CELL_LEN * 7;
    Secciones {
        aristas_cols,
        aristas_filas,
        etiquetas,
        estilos,
        celdas,
        cadenas,
    }
}

#[test]
fn ida_y_vuelta_del_fixture_determinista() {
    let dl = dibujo_dorado();
    let bytes = encode(&dl);
    assert_eq!(bytes.len(), encoded_len(&dl));
    assert_eq!(decode(&bytes).unwrap(), dl);
    // Codificar dos veces da lo mismo (determinista).
    assert_eq!(encode(&dl), bytes);
}

#[test]
fn ida_y_vuelta_de_un_bloque_real() {
    let dir = tempfile::tempdir().unwrap();
    let mut core = abrir(&crear_small(dir.path()));
    let dl = core
        .draw_block(DrawRequest {
            sheet: 0,
            x0: 0.0,
            y0: 0.0,
            w: 900.0,
            h: 300.0,
            margin_x: 0.0,
            margin_y: 0.0,
        })
        .unwrap();
    assert!(!dl.cells.is_empty());
    let bytes = Core::encode(&dl);
    assert_eq!(decode(&bytes).unwrap(), *dl);
}

#[test]
fn el_tamano_sigue_la_formula_del_plan() {
    let dl = dibujo_dorado();
    let s = secciones();
    assert_eq!(s.cadenas + dl.strings.len(), encode(&dl).len());
    // Todas las secciones empiezan en múltiplos de 4 sin relleno.
    for desp in [
        s.aristas_cols,
        s.aristas_filas,
        s.etiquetas,
        s.estilos,
        s.celdas,
        s.cadenas,
    ] {
        assert_eq!(desp % 4, 0, "sección en {desp}");
    }
    assert_eq!(HEADER_LEN, 64);
    assert_eq!((STYLE_LEN, CELL_LEN), (20, 20));
}

#[test]
fn la_cabecera_tiene_el_formato_del_plan() {
    let b = encode(&dibujo_dorado());
    assert_eq!(&b[0..4], b"PLD1", "magia «PLD1» en memoria");
    assert_eq!(u32::from_le_bytes(b[0..4].try_into().unwrap()), MAGIC);
    assert_eq!(MAGIC, 0x3144_4C50);
    assert_eq!(u16::from_le_bytes(b[4..6].try_into().unwrap()), VERSION);
    assert_eq!(VERSION, 1);
    assert_eq!(&b[6..8], &[0, 0], "banderas");
    assert_eq!(
        &b[8..16],
        &[8, 7, 6, 5, 4, 3, 2, 1],
        "data_version en little-endian"
    );
    let u32_en = |i: usize| u32::from_le_bytes(b[i..i + 4].try_into().unwrap());
    assert_eq!(u32_en(16), 2, "hoja");
    assert_eq!(u32_en(20) as i32, 1000, "first_row");
    assert_eq!(u32_en(24) as i32, 27, "first_col");
    assert_eq!(u32_en(28), 3, "n_rows");
    assert_eq!(u32_en(32), 4, "n_cols");
    assert_eq!(u32_en(36), 4, "n_styles");
    assert_eq!(u32_en(40), 7, "n_cells");
    assert_eq!(
        u32_en(44) as usize,
        dibujo_dorado().strings.len(),
        "strings_len"
    );
    assert_eq!(f64::from_le_bytes(b[48..56].try_into().unwrap()), 2340.0);
    assert_eq!(f64::from_le_bytes(b[56..64].try_into().unwrap()), 24_975.0);
    // Primer estilo: fg negro opaco 0x000000FF, sin relleno, 17,1875 px.
    let s = secciones();
    assert_eq!(u32_en(s.estilos), 0x0000_00FF);
    assert_eq!(u32_en(s.estilos + 4), 0, "0 = sin relleno");
    assert_eq!(
        f32::from_le_bytes(b[s.estilos + 8..s.estilos + 12].try_into().unwrap()),
        17.1875
    );
    // Tercer estilo: todas las banderas (1+2+4+8+16 = 31), centro, arriba.
    let e2 = s.estilos + 2 * STYLE_LEN;
    assert_eq!(u32_en(e2), 0x3366_99FF, "0xRRGGBBAA");
    assert_eq!(u32_en(e2 + 4), 0xFFFF_00FF);
    assert_eq!(&b[e2 + 12..e2 + 14], &[2, 0], "family");
    assert_eq!(
        &b[e2 + 14..e2 + 20],
        &[31, 1, 0, 0, 0, 0],
        "flags, h_align, v_align, reservado"
    );
}

#[test]
fn el_dorado_coincide_byte_a_byte() {
    let dorado = dorado();
    let actual = encode(&dibujo_dorado());
    // Sin `assert_eq!` sobre los vectores: si falla, mostraría cientos de bytes.
    let primera_diferencia = actual
        .iter()
        .zip(&dorado)
        .position(|(a, b)| a != b)
        .or_else(|| (actual.len() != dorado.len()).then_some(actual.len().min(dorado.len())));
    assert!(
        primera_diferencia.is_none(),
        "el formato cambió (primer byte distinto: {primera_diferencia:?}); si es a propósito, \
         suba VERSION y regenere el dorado"
    );
    assert_eq!(decode(&dorado).unwrap(), dibujo_dorado());
    assert_eq!(
        dorado.len(),
        64 + 20 + 16 + 56 + 80 + 140 + dibujo_dorado().strings.len()
    );
}

#[test]
#[ignore = "escribe tests/golden/drawlist_v1.bin; correr solo para regenerar el dorado a propósito"]
fn regenerar_dorado() {
    std::fs::write(RUTA_DORADO, encode(&dibujo_dorado())).unwrap();
}

#[test]
fn modificar_cualquier_byte_del_dorado_se_detecta() {
    let esperado = dibujo_dorado();
    let dorado = dorado();
    let (mut rechazados, mut distintos) = (0, 0);
    for i in 0..dorado.len() {
        for mascara in [0x01u8, 0x80, 0xFF] {
            let mut alterado = dorado.clone();
            alterado[i] ^= mascara;
            // 1) La prueba de bytes del dorado fallaría.
            assert!(encode(&esperado) != alterado);
            // 2) El lector o bien lo rechaza o bien devuelve algo distinto: nunca lo toma
            //    por el mismo dibujo.
            match decode(&alterado) {
                Err(_) => rechazados += 1,
                Ok(otro) => {
                    assert!(otro != esperado, "byte {i} (^{mascara:#x}) no cambia nada");
                    distintos += 1;
                }
            }
        }
    }
    println!(
        "{} bytes x 3 máscaras: {rechazados} rechazados, {distintos} decodifican a algo distinto",
        dorado.len()
    );
}

#[test]
fn magia_y_version_incorrectas() {
    let b = encode(&dibujo_dorado());
    assert_eq!(
        decode(&parchear(&b, 0, b"PLD2")).unwrap_err(),
        CodecError::MagiaIncorrecta(u32::from_le_bytes(*b"PLD2"))
    );
    assert_eq!(
        decode(&parchear(&b, 0, &[0, 0, 0, 0])).unwrap_err(),
        CodecError::MagiaIncorrecta(0)
    );
    assert_eq!(
        decode(&parchear(&b, 4, &2u16.to_le_bytes())).unwrap_err(),
        CodecError::VersionNoSoportada(2)
    );
    assert_eq!(
        decode(&parchear(&b, 4, &0u16.to_le_bytes())).unwrap_err(),
        CodecError::VersionNoSoportada(0)
    );
    assert_eq!(
        decode(&parchear(&b, 6, &[1, 0])).unwrap_err(),
        CodecError::Reservado("banderas del encabezado")
    );
}

#[test]
fn truncado_en_cualquier_longitud_es_error_sin_panico() {
    let b = encode(&dibujo_dorado());
    for corte in 0..b.len() {
        match decode(&b[..corte]) {
            Err(CodecError::Truncado { necesarios, hay }) => {
                assert_eq!(hay, corte);
                assert!(necesarios as usize > corte, "corte {corte}");
            }
            otro => panic!("corte en {corte}: se esperaba Truncado y salió {otro:?}"),
        }
    }
    assert!(decode(&[]).is_err());
}

#[test]
fn bytes_de_mas_tambien_es_error() {
    let mut b = encode(&dibujo_dorado());
    b.push(0);
    assert!(matches!(decode(&b), Err(CodecError::Tamano(_))));
}

#[test]
fn tamanos_limite_validos_hacen_ida_y_vuelta() {
    // Una sola fila con todas las columnas de la hoja.
    let ancho = bloque_vacio(1, 1, 1, MAX_COLS as u32);
    assert_eq!(decode(&encode(&ancho)).unwrap(), ancho);
    // Una sola columna con todas las filas de la hoja (1 048 576).
    let alto = bloque_vacio(1, 1, MAX_ROWS as u32, 1);
    let bytes = encode(&alto);
    assert_eq!(
        bytes.len(),
        64 + 8 + 4 * (MAX_ROWS as usize + 1) + 8 * (MAX_ROWS as usize + 1)
    );
    assert_eq!(decode(&bytes).unwrap(), alto);
    // El bloque que acaba justo en la última fila y columna.
    let esquina = bloque_vacio(MAX_ROWS - 1, MAX_COLS - 2, 2, 3);
    assert_eq!(decode(&encode(&esquina)).unwrap(), esquina);
}

#[test]
fn tamanos_fuera_de_limite_son_error() {
    let base = encode(&bloque_vacio(1, 1, 2, 2));
    let valor = |bytes: &[u8]| matches!(decode(bytes), Err(CodecError::Valor(_)));
    // n_rows / n_cols a cero.
    assert!(valor(&parchear(&base, 28, &0u32.to_le_bytes())));
    assert!(valor(&parchear(&base, 32, &0u32.to_le_bytes())));
    // Más filas o columnas que la hoja.
    assert!(valor(&parchear(
        &base,
        28,
        &(MAX_ROWS as u32 + 1).to_le_bytes()
    )));
    assert!(valor(&parchear(
        &base,
        32,
        &(MAX_COLS as u32 + 1).to_le_bytes()
    )));
    // El bloque se sale de la hoja por abajo o por la derecha.
    assert!(valor(&parchear(&base, 20, &MAX_ROWS.to_le_bytes())));
    assert!(valor(&parchear(&base, 24, &MAX_COLS.to_le_bytes())));
    // first_row / first_col menores que 1.
    assert!(valor(&parchear(&base, 20, &0i32.to_le_bytes())));
    assert!(valor(&parchear(&base, 24, &(-3i32).to_le_bytes())));
    // Más celdas que posiciones.
    assert!(matches!(
        decode(&parchear(&base, 40, &5u32.to_le_bytes())),
        Err(CodecError::Tamano(_))
    ));
}

#[test]
fn una_cabecera_mentirosa_no_reserva_memoria() {
    // La cabecera declara cientos de millones de elementos pero el búfer es de 64 bytes más
    // unos pocos: el lector compara con la longitud antes de reservar nada.
    let base = encode(&bloque_vacio(1, 1, 2, 2));
    for (campo, desp) in [("n_styles", 36), ("n_cells", 40), ("strings_len", 44)] {
        let mentira = parchear(&base, desp, &u32::MAX.to_le_bytes());
        match decode(&mentira) {
            // `n_cells` > posiciones se detecta antes; los otros, como bytes que faltan.
            Err(CodecError::Truncado { necesarios, hay }) => {
                assert!(necesarios > 4_000_000_000, "{campo}: {necesarios}");
                assert_eq!(hay, base.len());
            }
            Err(CodecError::Tamano(_)) => assert_eq!(campo, "n_cells"),
            otro => panic!("{campo}: {otro:?}"),
        }
    }
    // El peor caso legal de filas y columnas con un búfer diminuto.
    let mut gigante = parchear(&base, 28, &(MAX_ROWS as u32).to_le_bytes());
    gigante = parchear(&gigante, 32, &(MAX_COLS as u32).to_le_bytes());
    assert!(matches!(decode(&gigante), Err(CodecError::Truncado { .. })));
}

#[test]
fn valores_inexistentes_se_rechazan() {
    let b = encode(&dibujo_dorado());
    let s = secciones();
    let e0 = s.estilos; // primer estilo
    let c0 = s.celdas; // primera celda
    let c1 = s.celdas + CELL_LEN; // segunda celda
    let valor = |bytes: &[u8]| matches!(decode(bytes), Err(CodecError::Valor(_)));
    let reservado = |bytes: &[u8]| matches!(decode(bytes), Err(CodecError::Reservado(_)));

    // Estilos.
    assert!(valor(&parchear(&b, e0 + 15, &[3])), "h_align = 3");
    assert!(valor(&parchear(&b, e0 + 16, &[3])), "v_align = 3");
    assert!(
        reservado(&parchear(&b, e0 + 14, &[32])),
        "bandera 32 desconocida"
    );
    assert!(
        reservado(&parchear(&b, e0 + 17, &[1])),
        "primer byte reservado"
    );
    assert!(
        reservado(&parchear(&b, e0 + 19, &[1])),
        "tercer byte reservado"
    );
    assert!(
        valor(&parchear(&b, e0 + 8, &f32::NAN.to_le_bytes())),
        "tamaño NaN"
    );
    assert!(
        valor(&parchear(&b, e0 + 8, &(-1.0f32).to_le_bytes())),
        "tamaño negativo"
    );
    // Celdas.
    assert!(valor(&parchear(&b, c0 + 18, &[5])), "kind = 5");
    assert!(
        valor(&parchear(&b, c0 + 16, &4u16.to_le_bytes())),
        "estilo 4 de 4"
    );
    assert!(
        valor(&parchear(&b, c0, &3u32.to_le_bytes())),
        "row_off = n_rows"
    );
    assert!(
        valor(&parchear(&b, c0 + 4, &4u32.to_le_bytes())),
        "col_off = n_cols"
    );
    assert!(
        valor(&parchear(&b, c0 + 19, &[MAX_SPILL + 1])),
        "spill por encima del tope"
    );
    // Orden: la segunda celda (0,1) pasa a (0,0): repetida.
    assert!(
        valor(&parchear(&b, c1 + 4, &0u32.to_le_bytes())),
        "celda repetida"
    );
    // Cadenas: una referencia más allá del final y una que parte un carácter UTF-8.
    let fuera = (dibujo_dorado().strings.len() as u32 + 1).to_le_bytes();
    assert!(valor(&parchear(&b, c0 + 8, &fuera)), "str_off fuera");
    let mut parte_caracter = parchear(&b, c0 + 8, &0u32.to_le_bytes());
    // El texto de la primera celda empieza tras las siete etiquetas (2*4 + 4*3 = 20 bytes) y
    // su «ñ» ocupa los bytes 6-7 de su texto: una cadena que acabe en medio de la «ñ».
    let inicio_texto = 20u32;
    parte_caracter = parchear(&parte_caracter, c0 + 8, &inicio_texto.to_le_bytes());
    parte_caracter = parchear(&parte_caracter, c0 + 12, &7u32.to_le_bytes());
    assert!(valor(&parte_caracter), "termina en medio de «ñ»");
    // UTF-8 inválido en las cadenas.
    let mut roto = b.clone();
    roto[s.cadenas] = 0xFF;
    assert_eq!(decode(&roto).unwrap_err(), CodecError::Utf8);
    // Etiqueta fuera de la sección de cadenas.
    assert!(valor(&parchear(&b, s.etiquetas, &9999u32.to_le_bytes())));
    // Aristas: la primera no es cero, una decrece, una es NaN o infinita.
    assert!(valor(&parchear(&b, s.aristas_cols, &1.0f32.to_le_bytes())));
    assert!(
        valor(&parchear(&b, s.aristas_cols + 8, &10.0f32.to_le_bytes())),
        "decrece"
    );
    assert!(valor(&parchear(
        &b,
        s.aristas_filas + 4,
        &f32::NAN.to_le_bytes()
    )));
    assert!(valor(&parchear(
        &b,
        s.aristas_filas + 4,
        &f32::INFINITY.to_le_bytes()
    )));
    assert!(
        valor(&parchear(&b, s.aristas_cols, &(-0.0f32).to_le_bytes())),
        "-0.0 no es 0"
    );
    // Origen no finito.
    assert!(valor(&parchear(&b, 48, &f64::NAN.to_le_bytes())));
}

/// Generador congruencial lineal (sin dependencias) para entradas reproducibles.
struct Lcg(u64);
impl Lcg {
    fn siguiente(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }
}

#[test]
fn entradas_aleatorias_nunca_entran_en_panico() {
    let mut lcg = Lcg(2026);
    // Bytes totalmente aleatorios de longitudes variadas.
    for _ in 0..2_000 {
        let n = (lcg.siguiente() % 400) as usize;
        let bytes: Vec<u8> = (0..n).map(|_| lcg.siguiente() as u8).collect();
        let _ = decode(&bytes);
    }
    // Y el dorado con 1-6 bytes cambiados a valores al azar (con la cabecera intacta a veces).
    let mut aceptados = 0;
    let dorado = dorado();
    for _ in 0..5_000 {
        let mut b = dorado.clone();
        for _ in 0..1 + lcg.siguiente() % 6 {
            let pos = (lcg.siguiente() as usize) % b.len();
            b[pos] = lcg.siguiente() as u8;
        }
        if decode(&b).is_ok() {
            aceptados += 1;
        }
    }
    println!("mutaciones aleatorias aceptadas por el lector: {aceptados} de 5000");
}
