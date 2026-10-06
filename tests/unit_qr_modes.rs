use image::RgbaImage;
use labelize::barcodes::qrcode::{encode, encode_with_mode};
use labelize::elements::barcode_qr::{QrCharacterMode as Mode, QrErrorCorrectionLevel as Ec};
use rxing::common::DecoderRXingResult;

fn decode(image: &RgbaImage, magnification: u32) -> DecoderRXingResult {
    assert_eq!(image.width(), image.height());
    assert_eq!(image.width() % magnification, 0);
    let side = image.width() / magnification - 8;
    let matrix: Vec<Vec<bool>> = (0..side)
        .map(|y| {
            (0..side)
                .map(|x| image.get_pixel((x + 4) * magnification, (y + 4) * magnification)[3] != 0)
                .collect()
        })
        .collect();
    rxing::qrcode::decoder::qrcode_decoder::decode_bool_array(&matrix)
        .expect("independent QR decoder must recover the generated symbol")
}

#[test]
fn explicit_byte_and_alphanumeric_keep_the_requested_mode_for_numeric_payload() {
    // Physical Q01 control: the same twenty digits fit version 1 at Q in
    // Numeric mode, while forcing Byte produces version 2. Alpha also needs V2.
    let data = "12345678901234567890";
    let numeric = encode_with_mode(data, 1, Ec::Q, Mode::Numeric).unwrap();
    let alpha = encode_with_mode(data, 1, Ec::Q, Mode::Alphanumeric).unwrap();
    let binary = encode_with_mode(data, 1, Ec::Q, Mode::Binary).unwrap();
    assert_eq!(numeric.dimensions(), (29, 29));
    assert_eq!(alpha.dimensions(), (33, 33));
    assert_eq!(binary.dimensions(), (33, 33));
    assert_eq!(numeric, encode(data, 1, Ec::Q).unwrap());
    assert_ne!(alpha, binary);

    for (image, mode_indicator) in [(&numeric, 1), (&alpha, 2), (&binary, 4)] {
        let decoded = decode(image, 1);
        assert_eq!(decoded.getText(), data);
        // RXing 0.9 serializes the recovered QR format bits, not the level name:
        // L=01, M=00, Q=11, H=10, returned as decimal strings.
        assert_eq!(decoded.getECLevel(), "3");
        assert_eq!(decoded.getRawBytes()[0] >> 4, mode_indicator);
        if mode_indicator == 4 {
            assert_eq!(decoded.getByteSegments(), &vec![data.as_bytes().to_vec()]);
        } else {
            assert!(decoded.getByteSegments().is_empty());
        }
    }
}

#[test]
fn explicit_modes_round_trip_without_changing_correction_level() {
    // The independent decoder reports QR's two EC format bits as decimal text.
    for (level, expected_level) in [(Ec::L, "1"), (Ec::M, "0"), (Ec::Q, "3"), (Ec::H, "2")] {
        for (mode, data, mode_indicator) in [
            (Mode::Numeric, "0001234567890", 1),
            (Mode::Alphanumeric, "0123 ABC$%*+-./:", 2),
            (Mode::Binary, "ABC12345678901234567890abc", 4),
        ] {
            let image = encode_with_mode(data, 2, level, mode).unwrap();
            let decoded = decode(&image, 2);
            assert_eq!(decoded.getText(), data);
            assert_eq!(decoded.getECLevel(), expected_level);
            assert_eq!(decoded.getRawBytes()[0] >> 4, mode_indicator);
            if mode == Mode::Binary {
                assert_eq!(decoded.getByteSegments(), &vec![data.as_bytes().to_vec()]);
            }
        }
    }
}

#[test]
fn byte_mode_preserves_utf8_bytes_and_controls() {
    // This checks the string API's byte contract; it does not imply printer
    // charset/ECI support. Byte segments must contain exactly the UTF-8 input.
    let data = "A|B\0ä😀";
    let image = encode_with_mode(data, 1, Ec::M, Mode::Binary).unwrap();
    let decoded = decode(&image, 1);
    assert_eq!(decoded.getRawBytes()[0] >> 4, 4);
    assert_eq!(decoded.getByteSegments(), &vec![data.as_bytes().to_vec()]);
}

#[test]
fn invalid_explicit_payloads_are_rejected_instead_of_reinterpreted() {
    for data in ["12A", "-1", "1 2", "１２"] {
        let error = encode_with_mode(data, 1, Ec::L, Mode::Numeric).unwrap_err();
        assert!(error.contains("ASCII digits"), "{data:?}: {error}");
        assert!(encode(data, 1, Ec::L).is_ok());
    }
    for data in ["abc", "HELLO_", "()", "ä"] {
        let error = encode_with_mode(data, 1, Ec::L, Mode::Alphanumeric).unwrap_err();
        assert!(error.contains("alphanumeric mode"), "{data:?}: {error}");
        assert!(encode(data, 1, Ec::L).is_ok());
    }
    for data in ["ABC", "漢字"] {
        let error = encode_with_mode(data, 1, Ec::L, Mode::Kanji).unwrap_err();
        assert!(error.contains("Shift-JIS"));
    }
    for mode in [
        Mode::Automatic,
        Mode::Numeric,
        Mode::Alphanumeric,
        Mode::Binary,
        Mode::Kanji,
    ] {
        assert!(encode_with_mode("", 1, Ec::L, mode)
            .unwrap_err()
            .contains("empty content"));
    }
}

#[test]
fn overflowing_explicit_byte_payload_does_not_fall_back_to_numeric() {
    // A standard version-40 L symbol supports 2953 Byte characters. Automatic
    // Numeric encoding can fit this payload, but an explicit Byte request cannot.
    let data = "1".repeat(3000);
    assert!(encode_with_mode(&data, 1, Ec::L, Mode::Binary)
        .unwrap_err()
        .contains("data too long for the requested character mode"));
    assert!(encode(&data, 1, Ec::L).is_ok());
}

#[test]
fn automatic_entry_point_and_explicit_modes_share_scaling_and_quiet_zone() {
    for (mode, data) in [
        (Mode::Automatic, "ABC12345678901234567890abc"),
        (Mode::Numeric, "0123456789"),
        (Mode::Alphanumeric, "AC-42"),
        (Mode::Binary, "abcabaa"),
    ] {
        let unit = encode_with_mode(data, 1, Ec::H, mode).unwrap();
        if mode == Mode::Automatic {
            assert_eq!(unit, encode(data, 1, Ec::H).unwrap());
        }
        assert_eq!(unit, encode_with_mode(data, 0, Ec::H, mode).unwrap());
        assert_eq!(unit, encode_with_mode(data, -1, Ec::H, mode).unwrap());
        for mag in [1, 2, 4] {
            let image = encode_with_mode(data, mag, Ec::H, mode).unwrap();
            assert_eq!(image.width(), unit.width() * mag as u32);
            let decoded = decode(&image, mag as u32);
            assert_eq!(decoded.getText(), data);
            for (x, y, pixel) in image.enumerate_pixels() {
                let (mx, my) = (x / mag as u32, y / mag as u32);
                assert_eq!(pixel, unit.get_pixel(mx, my));
                if mx < 4 || my < 4 || mx >= unit.width() - 4 || my >= unit.height() - 4 {
                    assert_eq!(pixel[3], 0, "quiet zone must remain transparent");
                }
            }
        }
    }
}
