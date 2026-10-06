use image::GrayImage;
use labelize::{CompatibilityProfile as Profile, DrawerOptions, Renderer, ZplParser};
use rxing::common::DecoderRXingResult;

const Q01: &str = "^XA^FO10,10^BQN,2,2^FDQM,B002012345678901234567890^FS^XZ";

fn options() -> DrawerOptions {
    DrawerOptions {
        label_width_mm: 32.0,
        label_height_mm: 32.0,
        dpmm: 8,
        ..Default::default()
    }
}

fn render(zpl: &str, profile: Option<Profile>) -> Result<Vec<u8>, String> {
    let label = ZplParser::new().parse(zpl.as_bytes())?.remove(0);
    let mut png = Vec::new();
    let renderer = Renderer::new();
    match profile {
        Some(profile) => {
            renderer.draw_label_as_png_with_profile(&label, &mut png, options(), profile)?
        }
        None => renderer.draw_label_as_png(&label, &mut png, options())?,
    }
    Ok(png)
}

fn pixels(png: &[u8]) -> GrayImage {
    image::load_from_memory(png).unwrap().to_luma8()
}

fn modules(png: &[u8]) -> Vec<Vec<bool>> {
    let image = pixels(png);
    let dark: Vec<_> = image
        .enumerate_pixels()
        .filter(|(_, _, p)| p[0] < 128)
        .collect();
    let min_x = dark.iter().map(|p| p.0).min().unwrap();
    let max_x = dark.iter().map(|p| p.0).max().unwrap();
    let min_y = dark.iter().map(|p| p.1).min().unwrap();
    let max_y = dark.iter().map(|p| p.1).max().unwrap();
    assert_eq!(max_x - min_x, max_y - min_y);
    let side = (max_x - min_x + 1) / 2;
    (0..side)
        .map(|y| {
            (0..side)
                .map(|x| image.get_pixel(min_x + 2 * x, min_y + 2 * y)[0] < 128)
                .collect()
        })
        .collect()
}

fn decode(png: &[u8]) -> (u32, DecoderRXingResult) {
    let matrix = modules(png);
    (
        matrix.len() as u32,
        rxing::qrcode::decoder::qrcode_decoder::decode_bool_array(&matrix).unwrap(),
    )
}

#[test]
fn q01_profiles_preserve_the_recorded_labelary_and_explicit_byte_matrices() {
    // The Byte fixture is the PR52 matrix visually matched by the user to the
    // native print; it is not represented as an independently decoded scan.
    for (profile, expected) in [
        (
            Profile::Labelary,
            include_str!("../testdata/qr-profiles/q01-labelary.txt"),
        ),
        (
            Profile::ZebraExperimental,
            include_str!("../testdata/qr-profiles/q01-explicit-byte.txt"),
        ),
    ] {
        let expected: Vec<Vec<bool>> = expected
            .lines()
            .map(|row| row.bytes().map(|bit| bit == b'1').collect())
            .collect();
        assert_eq!(
            modules(&render(Q01, Some(profile)).unwrap()),
            expected,
            "{profile}: full Q01 matrix"
        );
    }
}

#[test]
fn public_names_are_strict_and_labelary_remains_the_default() {
    assert_eq!(Profile::default(), Profile::Labelary);
    for (name, profile) in [
        ("labelary", Profile::Labelary),
        ("zebra-experimental", Profile::ZebraExperimental),
    ] {
        assert_eq!(name.parse::<Profile>().unwrap(), profile);
        assert_eq!(profile.as_str(), name);
        assert_eq!(profile.to_string(), name);
    }
    for name in ["", "zebra", "Labelary", " labelary", "vector", "debug"] {
        assert!(name.parse::<Profile>().is_err(), "{name:?}");
    }
}

#[test]
fn q01_changes_only_when_zebra_is_requested_and_preserves_the_payload() {
    let default = render(Q01, None).unwrap();
    let labelary = render(Q01, Some(Profile::Labelary)).unwrap();
    let zebra = render(Q01, Some(Profile::ZebraExperimental)).unwrap();
    assert_eq!(default, labelary, "existing API must remain byte-identical");
    let (labelary_side, labelary_decoded) = decode(&labelary);
    let (zebra_side, zebra_decoded) = decode(&zebra);
    assert_eq!(labelary_side, 21, "automatic Numeric fits version 1");
    assert_eq!(zebra_side, 25, "explicit Byte requires version 2");
    assert_eq!(labelary_decoded.getRawBytes()[0] >> 4, 1);
    assert_eq!(zebra_decoded.getRawBytes()[0] >> 4, 4);
    for result in [&labelary_decoded, &zebra_decoded] {
        assert_eq!(result.getText(), "12345678901234567890");
        // rxing exposes the QR format-bit value as a string (Q = 0b11).
        assert_eq!(result.getECLevel(), "3");
    }
    assert_eq!(
        zebra_decoded.getByteSegments(),
        &vec![b"12345678901234567890".to_vec()]
    );
}

#[test]
fn explicit_numeric_and_alpha_and_byte_modes_reach_the_renderer() {
    for (field, indicator, data) in [
        ("QM,N0123456789", 1, "0123456789"),
        ("QM,A1234567890", 2, "1234567890"),
        ("QM,B0003012", 4, "012"),
        ("QM,B0003A|B", 4, "A|B"),
    ] {
        let zpl = format!("^XA^FO10,10^BQN,2,2^FD{field}^FS^XZ");
        let (_, result) = decode(&render(&zpl, Some(Profile::ZebraExperimental)).unwrap());
        assert_eq!(result.getRawBytes()[0] >> 4, indicator);
        assert_eq!(result.getText(), data);
    }
}

#[test]
fn automatic_fields_and_empty_fields_keep_both_profiles_identical() {
    for field in [
        "QA,12345678901234567890",
        "HA,ABC12345678901234567890abc",
        "",
        "QA,",
    ] {
        let zpl = format!("^XA^FO10,10^BQN,2,2^FD{field}^FS^XZ");
        let default = render(&zpl, None).unwrap();
        assert_eq!(default, render(&zpl, Some(Profile::Labelary)).unwrap());
        assert_eq!(
            default,
            render(&zpl, Some(Profile::ZebraExperimental)).unwrap()
        );
    }
}

#[test]
fn invalid_explicit_modes_do_not_silently_fall_back_even_with_reverse_print() {
    for (field, message) in [
        ("QM,N12A", "ASCII digits"),
        ("QM,Aabc", "alphanumeric"),
        ("QM,KABC", "Shift-JIS"),
    ] {
        for reverse in ["", "^FR"] {
            let zpl = format!("^XA^FO10,10{reverse}^BQN,2,2^FD{field}^FS^XZ");
            assert!(render(&zpl, None).is_ok());
            assert!(render(&zpl, Some(Profile::Labelary)).is_ok());
            let error = render(&zpl, Some(Profile::ZebraExperimental)).unwrap_err();
            assert!(error.contains(message), "{error}");
        }
    }
}

#[test]
fn profile_is_per_call_and_does_not_change_shared_geometry() {
    let labels = ZplParser::new().parse(Q01.as_bytes()).unwrap();
    let renderer = Renderer; // Preserve existing unit-struct construction.
    let mut baseline = Vec::new();
    renderer
        .draw_label_as_png(&labels[0], &mut baseline, options())
        .unwrap();
    for profile in [
        Profile::ZebraExperimental,
        Profile::Labelary,
        Profile::ZebraExperimental,
    ] {
        let mut png = Vec::new();
        renderer
            .draw_label_as_png_with_profile(&labels[0], &mut png, options(), profile)
            .unwrap();
        assert_eq!(png == baseline, profile == Profile::Labelary);
    }
    let mut after = Vec::new();
    renderer
        .draw_label_as_png(&labels[0], &mut after, options())
        .unwrap();
    assert_eq!(after, baseline);

    // PR48 constraints and ordinary graphics are common policies. Only QR
    // character interpretation changes in this first experimental profile.
    let geometry = "^XA^FO10,10^GB50,40,2^FS^FO80,10^GC40,2^FS^FO10,80^BXN,2,200,36,12,6,,2^FDABCD12345678901234567890efgh^FS^FO10,150^BXN,2,200,0,12,6,,2^FDABCD12345678901234567890efgh^FS^XZ";
    assert_eq!(
        render(geometry, None).unwrap(),
        render(geometry, Some(Profile::ZebraExperimental)).unwrap()
    );
}

#[test]
fn both_profiles_keep_pr53_independent_labelary_matrices() {
    let zpl = std::fs::read("testdata/unit/qr_mask_selection.zpl").unwrap();
    let labels = ZplParser::new().parse(&zpl).unwrap();
    let reference = image::open("testdata/unit/qr_mask_selection.png")
        .unwrap()
        .to_luma8();
    let opts = DrawerOptions {
        label_width_mm: 101.625,
        label_height_mm: 203.25,
        dpmm: 8,
        ..Default::default()
    };
    for profile in [Profile::Labelary, Profile::ZebraExperimental] {
        let mut png = Vec::new();
        Renderer::new()
            .draw_label_as_png_with_profile(&labels[0], &mut png, opts.clone(), profile)
            .unwrap();
        assert_eq!(
            pixels(&png),
            reference,
            "{profile}: preserve all five PR53 matrices"
        );
    }
}
