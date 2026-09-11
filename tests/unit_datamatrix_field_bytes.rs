use labelize::barcodes::datamatrix_legacy;
use labelize::elements::label_element::LabelElement;
use labelize::{DrawerOptions, LabelInfo, Renderer, ZplParser};
use std::io::Cursor;

fn barcode(
    label: &LabelInfo,
) -> &labelize::elements::barcode_datamatrix::BarcodeDatamatrixWithData {
    match &label.elements[0] {
        LabelElement::BarcodeDatamatrix(bc) => bc,
        other => panic!("expected DataMatrix, got {other:?}"),
    }
}

fn assert_rendered_bytes(label: &LabelInfo, expected: &[u8], quality: u16) {
    assert_eq!(barcode(label).data_bytes.as_deref(), Some(expected));
    let mut output = Cursor::new(Vec::new());
    Renderer::new()
        .draw_label_as_png(label, &mut output, DrawerOptions::default())
        .unwrap();
    let actual = image::load_from_memory(output.get_ref())
        .unwrap()
        .to_rgba8();
    let matrix = datamatrix_legacy::encode_with_ecc(expected, 6, quality, None).unwrap();
    for y in 0..matrix.height() * 2 {
        for x in 0..matrix.width() * 2 {
            assert_eq!(
                actual.get_pixel(20 + x as u32, 20 + y as u32)[0] == 0,
                matrix.get(x / 2, y / 2)
            );
        }
    }
}

#[test]
fn all_fh_byte_values_survive_without_utf8_reinterpretation_or_resplitting() {
    let expected: Vec<u8> = (0..=255).collect();
    let field: String = expected.iter().map(|b| format!("_{b:02X}")).collect();
    let source = format!("^XA^FO20,20^BXN,2,0^FH_^FD{field}^FS^XZ");
    let labels = ZplParser::new().parse(source.as_bytes()).unwrap();
    assert_rendered_bytes(&labels[0], &expected, 0);
}

#[test]
fn identical_display_strings_do_not_collapse_different_field_bytes() {
    let source = b"^XA^FO20,20^BXN,2,0^FH_^FD_E4^FS^XZ^XA^FO20,20^BXN,2,0^FH_^FD_C3_A4^FS^XZ";
    let labels = ZplParser::new().parse(source).unwrap();
    assert_eq!(barcode(&labels[0]).data, barcode(&labels[1]).data);
    assert_rendered_bytes(&labels[0], &[0xe4], 0);
    assert_rendered_bytes(&labels[1], &[0xc3, 0xa4], 0);
}

#[test]
fn raw_invalid_utf8_is_preserved_for_every_legacy_quality() {
    for quality in [0, 50, 80, 100, 140] {
        let mut source = format!("^XA^FO20,20^BXN,2,{quality}^FD").into_bytes();
        source.extend_from_slice(&[0x80, 0xff, 0xc3, 0xa4]);
        source.extend_from_slice(b"^FS^XZ");
        let labels = ZplParser::new().parse(&source).unwrap();
        assert_rendered_bytes(&labels[0], &[0x80, 0xff, 0xc3, 0xa4], quality);
    }
}

#[test]
fn field_bytes_survive_stored_format_recall() {
    let source = b"^XA^DFR:LEG.ZPL^FO20,20^BXN,2,0^FN1^FS^XZ\
                   ^XA^XFR:LEG.ZPL^FN1^FH_^FD_80_FF^FS^XZ";
    let labels = ZplParser::new().parse(source).unwrap();
    assert_eq!(labels.len(), 1);
    assert_rendered_bytes(&labels[0], &[0x80, 0xff], 0);
}

#[test]
fn custom_command_prefix_and_fh_indicator_preserve_field_bytes() {
    let source = b"^XA^CC!!FO20,20!BXN,2,0!FH#!FD#80#5E#7E!FS!XZ";
    let labels = ZplParser::new().parse(source).unwrap();
    assert_rendered_bytes(&labels[0], &[0x80, b'^', b'~'], 0);
}

#[test]
fn field_bytes_and_fh_indicator_reset_between_labels_and_parser_calls() {
    let mut parser = ZplParser::new();
    let first = parser
        .parse(b"^XA^FO20,20^BXN,2,0^FH_^FD_FF^FS^XZ")
        .unwrap();
    assert_rendered_bytes(&first[0], &[0xff], 0);
    let second = parser.parse(b"^XA^FO20,20^BXN,2,0^FD_FF^FS^XZ").unwrap();
    assert_rendered_bytes(&second[0], b"_FF", 0);
}

#[test]
fn malformed_hex_is_literal_and_fv_keeps_bytes() {
    assert_eq!(
        labelize::hex::decode_escaped_bytes(b"_GG_4_80", b'_'),
        b"_GG_4\x80"
    );
    let labels = ZplParser::new()
        .parse(b"^XA^FO20,20^BXN,2,0^FH_^FV_80_FF^FS^XZ")
        .unwrap();
    assert_rendered_bytes(&labels[0], &[0x80, 0xff], 0);
}

#[test]
fn documented_legacy_escapes_are_single_pass_and_do_not_apply_ecc200_rules() {
    for (input, expected) in [
        (&b"A\\&B"[..], &b"A\r\nB"[..]),
        (&b"\\&\\&"[..], &b"\r\n\r\n"[..]),
        (&b"A\\\\B"[..], &b"A\\B"[..]),
        (&b"\\\\&"[..], &b"\\&"[..]),
        (&b"A\\"[..], &b"A\\"[..]),
        (&b"_1ABC|Z\\q"[..], &b"_1ABC|Z\\q"[..]),
    ] {
        assert_eq!(
            datamatrix_legacy::prepare_zpl_field(input).unwrap(),
            expected
        );
    }
    assert!(datamatrix_legacy::prepare_zpl_field(b"A||B").is_err());
}

fn rendered(source: &[u8]) -> Result<Vec<u8>, String> {
    let labels = ZplParser::new().parse(source)?;
    let mut output = Cursor::new(Vec::new());
    Renderer::new().draw_label_as_png(&labels[0], &mut output, DrawerOptions::default())?;
    Ok(output.into_inner())
}

#[test]
fn legacy_crlf_escapes_match_explicit_fh_control_bytes_for_all_qualities() {
    for quality in [0, 50, 80, 100, 140] {
        let prefix = format!("^XA^CI13^FO20,20^BXN,2,{quality},0,0,6");
        let escaped = rendered(format!("{prefix}^FDAB\\&12\\\\Z\\&^FS^XZ").as_bytes()).unwrap();
        let hex =
            rendered(format!("{prefix}^FH_^FDAB_0D_0A12_5CZ_0D_0A^FS^XZ").as_bytes()).unwrap();
        assert_eq!(escaped, hex);
    }
    for format in 1..=4 {
        assert!(rendered(format!("^XA^BXN,2,0,0,0,{format}^FD12\\&34^FS^XZ").as_bytes()).is_err());
    }
}

#[test]
fn ecc200_does_not_use_legacy_field_substitutions() {
    let escaped = rendered(b"^XA^FO20,20^BXN,2,200^FDAB\\&12^FS^XZ").unwrap();
    let crlf = rendered(b"^XA^FO20,20^BXN,2,200^FH_^FDAB_0D_0A12^FS^XZ").unwrap();
    assert_ne!(escaped, crlf);
}
