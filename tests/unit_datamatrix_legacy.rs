use labelize::barcodes::{datamatrix_legacy, BitMatrix};
use labelize::{DrawerOptions, Renderer, ZplParser};
use std::io::Cursor;

#[test]
fn placement_matches_all_21_fixed_norm_grids_and_reuses_immutable_cache() {
    let mut count = 0;
    for line in include_str!("../testdata/legacy/placement-grids.txt").lines() {
        let (size, values) = line.split_once(':').unwrap();
        let size = size.parse::<usize>().unwrap();
        let expected: Vec<usize> = values.split(',').map(|v| v.parse().unwrap()).collect();
        let actual = datamatrix_legacy::placement_for_size(size).unwrap();
        assert_eq!(actual, expected, "symbol size {size}");
        assert!(std::ptr::eq(
            actual,
            datamatrix_legacy::placement_for_size(size).unwrap()
        ));
        let mut sorted = actual.to_vec();
        sorted.sort_unstable();
        assert_eq!(sorted, (0..actual.len()).collect::<Vec<_>>());
        count += actual.len();
    }
    assert_eq!(count, 18_389);
}

fn check_matrices(fixtures: &str) {
    let normalized = fixtures.replace("\r\n", "\n");
    for fixture in normalized.trim().split("\n\n") {
        let mut lines = fixture.lines();
        let header: Vec<_> = lines.next().unwrap().split('|').collect();
        let format = header[0].parse().unwrap();
        let size = header[1].parse().unwrap();
        let bytes: Vec<u8> = header[2]
            .as_bytes()
            .chunks(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect();
        let matrix = datamatrix_legacy::encode(&bytes, format, Some(size)).unwrap();
        let rows: Vec<_> = lines.collect();
        assert_eq!(matrix.width(), rows.len());
        for (y, row) in rows.iter().enumerate() {
            assert_eq!(matrix.width(), row.len());
            for (x, value) in row.bytes().enumerate() {
                assert_eq!(
                    matrix.get(x, y),
                    value == b'1',
                    "size {size}, format {format}, ({x},{y})"
                );
            }
        }
    }
}

#[test]
fn recorded_printer_samples_match_module_for_module() {
    check_matrices(include_str!(
        "../testdata/legacy/ecc000-printer-matrices.txt"
    ));
}

#[test]
fn javascript_port_vectors_match_without_claiming_independent_validation() {
    check_matrices(include_str!(
        "../testdata/legacy/ecc000-js-port-vectors.txt"
    ));
}

fn render(zpl: &[u8]) -> Result<image::RgbaImage, String> {
    let labels = ZplParser::new().parse(zpl).unwrap();
    let mut output = Cursor::new(Vec::new());
    Renderer::new().draw_label_as_png(&labels[0], &mut output, DrawerOptions::default())?;
    Ok(image::load_from_memory(output.get_ref())
        .unwrap()
        .to_rgba8())
}

fn assert_drawn(matrix: &BitMatrix, orientation: &str) {
    let source = format!("^XA^FO20,20^BX{orientation},2,0,13,17,6^FDAB12^FS^XZ");
    let image = render(source.as_bytes()).unwrap();
    let expected = matrix.to_image(2, 2);
    let expected = match orientation {
        "R" => image::imageops::rotate90(&expected),
        "I" => image::imageops::rotate180(&expected),
        "B" => image::imageops::rotate270(&expected),
        _ => expected,
    };
    for y in 0..expected.height() {
        for x in 0..expected.width() {
            assert_eq!(
                image.get_pixel(20 + x, 20 + y)[0] == 0,
                expected.get_pixel(x, y)[3] != 0,
                "orientation {orientation}, ({x},{y})"
            );
        }
    }
}

#[test]
fn zpl_routes_ecc000_dimensions_scaling_and_all_orientations() {
    let matrix = datamatrix_legacy::encode(b"AB12", 6, Some(17)).unwrap();
    for orientation in ["N", "R", "I", "B"] {
        assert_drawn(&matrix, orientation);
    }
    let automatic = render(b"^XA^FO20,20^BXN,2,0^FDAB12^FS^XZ").unwrap();
    assert_eq!(
        automatic,
        render(b"^XA^FO20,20^BXN,2,0,50,51^FDAB12^FS^XZ").unwrap()
    );
}

#[test]
fn zpl_keeps_ecc200_escapes_literal_for_legacy_and_handles_ascii_fh() {
    let plain = render(b"^XA^FO20,20^BXN,2,0^FD_1ABC^FS^XZ").unwrap();
    assert_eq!(
        plain,
        render(b"^XA^FO20,20^BXN,2,0,0,0,6,!^FD_1ABC^FS^XZ").unwrap()
    );
    let fh = render(b"^XA^FO20,20^BXN,2,0^FH_^FDAB_1D12^FS^XZ").unwrap();
    let literal = render(b"^XA^FO20,20^BXN,2,0^FDAB\x1d12^FS^XZ").unwrap();
    assert_eq!(fh, literal);
}

#[test]
fn zpl_unsupported_or_invalid_legacy_fields_fail_explicitly() {
    for source in [
        "^XA^BXN,2,0,10^FDAB12^FS^XZ",
        "^XA^BXN,2,0,9,9^FDAB12^FS^XZ",
        "^XA^BXN,2,0,0,0,1^FDAB12^FS^XZ",
        "^XA^BXN,2,0,0,0,7^FDAB12^FS^XZ",
        "^XA^BXN,2,0,0,0,6,_,2^FDAB12^FS^XZ",
        "^XA^BXN,2,0^FDAB\\&12^FS^XZ",
        "^XA^BXN,2,0^FDAB||12^FS^XZ",
        "^XA^BXN,2,0^FDä^FS^XZ",
    ] {
        assert!(render(source.as_bytes()).is_err(), "{source}");
    }
}
