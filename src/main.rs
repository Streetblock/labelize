#[cfg(feature = "cli")]
use std::fs;
#[cfg(feature = "cli")]
use std::io::Cursor;
#[cfg(feature = "cli")]
use std::path::{Path, PathBuf};

#[cfg(feature = "cli")]
use clap::{Parser, Subcommand, ValueEnum};
#[cfg(any(feature = "cli", feature = "serve"))]
use labelize::{CompatibilityProfile, DrawerOptions, EplParser, LabelInfo, Renderer, ZplParser};

#[cfg(feature = "cli")]
#[derive(Parser)]
#[command(
    name = "labelize",
    version,
    about = "Turn ZPL/EPL into pixels — label rendering, simplified."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[cfg(feature = "cli")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum InputFormat {
    Zpl,
    Epl,
}

#[cfg(feature = "cli")]
#[derive(Clone, Copy, ValueEnum)]
enum OutputType {
    Png,
    Pdf,
}

#[cfg(feature = "cli")]
#[derive(Subcommand)]
enum Commands {
    /// Convert a ZPL/EPL file to PNG or PDF
    Convert {
        /// Input file path (.zpl or .epl)
        input: PathBuf,

        /// Output file path (default: input stem + .png/.pdf)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Input format (auto-detected from extension if omitted)
        #[arg(short, long)]
        format: Option<InputFormat>,

        /// Output type
        #[arg(short = 't', long = "type", default_value = "png")]
        output_type: OutputType,

        /// Label width in mm
        #[arg(long, default_value_t = 102.0)]
        width: f64,

        /// Label height in mm
        #[arg(long, default_value_t = 152.0)]
        height: f64,

        /// Dots per mm (6, 8, 12, or 24)
        #[arg(long, default_value_t = 8)]
        dpmm: i32,

        /// Emit 8-bit grayscale output preserving antialiasing (default: 1-bit)
        #[arg(long)]
        antialias: bool,

        /// Command interpretation: labelary or zebra-experimental (ZPL only)
        #[arg(long, default_value = "labelary", value_name = "PROFILE")]
        profile: CompatibilityProfile,
    },

    /// Start HTTP server for label conversion
    #[cfg(feature = "serve")]
    Serve {
        /// Host to bind to
        #[arg(long, default_value = "0.0.0.0")]
        host: String,

        /// Port to listen on
        #[arg(short, long, default_value_t = 8080)]
        port: u16,

        /// Default command interpretation for requests without a profile query
        #[arg(long, default_value = "labelary", value_name = "PROFILE")]
        default_profile: CompatibilityProfile,
    },
}

#[cfg(feature = "cli")]
fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Convert {
            input,
            output,
            format,
            output_type,
            width,
            height,
            dpmm,
            antialias,
            profile,
        } => {
            if let Err(e) = convert_file(
                &input,
                output.as_deref(),
                format,
                output_type,
                width,
                height,
                dpmm,
                antialias,
                profile,
            ) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        #[cfg(feature = "serve")]
        Commands::Serve {
            host,
            port,
            default_profile,
        } => {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
            rt.block_on(serve(host, port, default_profile));
        }
    }
}

#[cfg(not(feature = "cli"))]
fn main() {
    eprintln!("CLI not available. Rebuild with: cargo build --features cli");
    std::process::exit(1);
}

#[cfg(feature = "cli")]
fn detect_format(path: &Path, override_fmt: Option<InputFormat>) -> InputFormat {
    if let Some(fmt) = override_fmt {
        return fmt;
    }
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    match ext.to_lowercase().as_str() {
        "epl" => InputFormat::Epl,
        _ => InputFormat::Zpl,
    }
}

#[cfg(feature = "cli")]
fn parse_labels(content: &[u8], format: InputFormat, dpmm: i32) -> Result<Vec<LabelInfo>, String> {
    match format {
        InputFormat::Epl => EplParser::new().parse(content),
        // dpmm up front: ^MU may express coordinates in mm/inches.
        InputFormat::Zpl => ZplParser::with_dpmm(dpmm).parse(content),
    }
}

#[cfg(feature = "cli")]
fn validate_profile(format: InputFormat, profile: CompatibilityProfile) -> Result<(), String> {
    if matches!(format, InputFormat::Epl)
        && matches!(profile, CompatibilityProfile::ZebraExperimental)
    {
        return Err("The zebra-experimental profile supports ZPL input only".to_string());
    }
    Ok(())
}

#[cfg(feature = "cli")]
fn output_extension(output_type: OutputType) -> &'static str {
    match output_type {
        OutputType::Png => "png",
        OutputType::Pdf => "pdf",
    }
}

#[cfg(feature = "cli")]
fn default_output_path(input: &Path, output_type: OutputType, index: Option<usize>) -> PathBuf {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let ext = output_extension(output_type);
    let parent = input.parent().unwrap_or_else(|| Path::new("."));
    match index {
        Some(i) => parent.join(format!("{}_{}.{}", stem, i + 1, ext)),
        None => parent.join(format!("{}.{}", stem, ext)),
    }
}

#[cfg(feature = "cli")]
fn render_label(
    label: &LabelInfo,
    options: &DrawerOptions,
    output_type: OutputType,
    profile: CompatibilityProfile,
) -> Result<Vec<u8>, String> {
    let renderer = Renderer::new();
    let mut buf = Cursor::new(Vec::new());
    match output_type {
        OutputType::Png => {
            renderer.draw_label_as_png_with_profile(label, &mut buf, options.clone(), profile)?
        }
        OutputType::Pdf => {
            renderer.draw_label_as_png_with_profile(label, &mut buf, options.clone(), profile)?;
            let img = image::load_from_memory(&buf.into_inner())
                .map_err(|e| format!("Failed to decode rendered image: {}", e))?
                .to_rgba8();
            let mut pdf_buf = Cursor::new(Vec::new());
            labelize::encode_pdf(&img, options, &mut pdf_buf)
                .map_err(|e| format!("Failed to encode PDF: {}", e))?;
            return Ok(pdf_buf.into_inner());
        }
    }
    Ok(buf.into_inner())
}

#[cfg(feature = "cli")]
#[allow(clippy::too_many_arguments)]
fn convert_file(
    input: &Path,
    output: Option<&Path>,
    format: Option<InputFormat>,
    output_type: OutputType,
    width: f64,
    height: f64,
    dpmm: i32,
    antialias: bool,
    profile: CompatibilityProfile,
) -> Result<(), String> {
    let fmt = detect_format(input, format);
    validate_profile(fmt, profile)?;
    let content = fs::read(input).map_err(|e| format!("Failed to read input file: {}", e))?;
    let labels = parse_labels(&content, fmt, dpmm)?;

    if labels.is_empty() {
        return Err("No labels found in input".to_string());
    }

    let options = DrawerOptions {
        label_width_mm: width,
        label_height_mm: height,
        dpmm,
        antialias,
        ..Default::default()
    };

    let multi = labels.len() > 1;
    for (i, label) in labels.iter().enumerate() {
        let out_path = match output {
            Some(p) if !multi => p.to_path_buf(),
            Some(p) => {
                let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
                let ext = p
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or(output_extension(output_type));
                let parent = p.parent().unwrap_or_else(|| Path::new("."));
                parent.join(format!("{}_{}.{}", stem, i + 1, ext))
            }
            None => default_output_path(input, output_type, if multi { Some(i) } else { None }),
        };

        let data = render_label(label, &options, output_type, profile)?;
        fs::write(&out_path, data).map_err(|e| format!("Failed to write output file: {}", e))?;
        println!("Converted {} -> {}", input.display(), out_path.display());
    }

    Ok(())
}

#[cfg(feature = "serve")]
async fn serve(host: String, port: u16, default_profile: CompatibilityProfile) {
    use axum::{
        http::{header, StatusCode},
        response::IntoResponse,
        routing::{get, post},
        Router,
    };

    async fn playground_page() -> impl IntoResponse {
        (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "text/html; charset=utf-8"),
                (header::CACHE_CONTROL, "no-cache"),
            ],
            labelize::playground::PLAYGROUND_HTML,
        )
    }

    async fn health() -> impl IntoResponse {
        (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            r#"{"status":"ok"}"#,
        )
    }

    let app = Router::new()
        .route("/", get(playground_page))
        .route("/health", get(health))
        .route("/convert", post(convert_handler))
        .with_state(default_profile);

    let addr = format!("{}:{}", host, port);
    println!("Starting server on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind");
    axum::serve(listener, app).await.expect("Server failed");
}

#[cfg(feature = "serve")]
#[derive(serde::Deserialize)]
struct ConvertParams {
    #[serde(default = "default_width")]
    width: f64,
    #[serde(default = "default_height")]
    height: f64,
    #[serde(default = "default_dpmm")]
    dpmm: i32,
    #[serde(default)]
    output: Option<String>,
    /// Emit 8-bit grayscale output preserving antialiasing.
    #[serde(default)]
    antialias: bool,
    #[serde(default)]
    profile: Option<String>,
}

#[cfg(feature = "serve")]
fn default_width() -> f64 {
    102.0
}

#[cfg(feature = "serve")]
fn default_height() -> f64 {
    152.0
}

#[cfg(feature = "serve")]
fn default_dpmm() -> i32 {
    8
}

#[cfg(feature = "serve")]
async fn convert_handler(
    axum::extract::State(default_profile): axum::extract::State<CompatibilityProfile>,
    headers: axum::http::HeaderMap,
    axum::extract::Query(params): axum::extract::Query<ConvertParams>,
    body: axum::body::Bytes,
) -> axum::response::Response {
    use axum::{
        http::{header, StatusCode},
        response::IntoResponse,
    };

    let profile = match params.profile.as_deref() {
        Some(value) => match value.parse::<CompatibilityProfile>() {
            Ok(profile) => profile,
            Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
        },
        None => default_profile,
    };
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let format = if content_type.contains("epl") {
        InputFormat::Epl
    } else {
        InputFormat::Zpl
    };
    if let Err(e) = validate_profile(format, profile) {
        return (StatusCode::BAD_REQUEST, e).into_response();
    }

    let labels = match parse_labels(&body, format, params.dpmm) {
        Ok(labels) => labels,
        Err(e) => return (StatusCode::BAD_REQUEST, e).into_response(),
    };
    let label = match labels.into_iter().next() {
        Some(label) => label,
        None => return (StatusCode::BAD_REQUEST, "No labels found").into_response(),
    };
    let options = DrawerOptions {
        label_width_mm: params.width,
        label_height_mm: params.height,
        dpmm: params.dpmm,
        antialias: params.antialias,
        ..Default::default()
    };
    let (output_type, content_type) = if params.output.as_deref() == Some("pdf") {
        (OutputType::Pdf, "application/pdf")
    } else {
        (OutputType::Png, "image/png")
    };
    match render_label(&label, &options, output_type, profile) {
        Ok(data) => (StatusCode::OK, [(header::CONTENT_TYPE, content_type)], data).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

#[cfg(all(test, feature = "cli"))]
mod tests {
    use super::*;

    const EXPLICIT_BYTE_QR: &[u8] = b"^XA^FO10,10^BQN,2,2^FDQM,B002012345678901234567890^FS^XZ";

    #[test]
    fn cli_defaults_to_labelary_and_accepts_explicit_profiles() {
        for (args, expected) in [
            (
                vec!["labelize", "convert", "label.zpl"],
                CompatibilityProfile::Labelary,
            ),
            (
                vec!["labelize", "convert", "label.zpl", "--profile", "labelary"],
                CompatibilityProfile::Labelary,
            ),
            (
                vec![
                    "labelize",
                    "convert",
                    "label.zpl",
                    "--profile",
                    "zebra-experimental",
                ],
                CompatibilityProfile::ZebraExperimental,
            ),
        ] {
            let cli = Cli::try_parse_from(args).unwrap();
            match cli.command {
                Commands::Convert { profile, .. } => assert_eq!(profile, expected),
                #[cfg(feature = "serve")]
                Commands::Serve { .. } => panic!("Expected convert command"),
            }
        }
        assert!(
            Cli::try_parse_from(["labelize", "convert", "label.zpl", "--profile", "unknown",])
                .is_err()
        );
    }

    #[cfg(feature = "serve")]
    #[test]
    fn server_startup_parses_default_profile() {
        for (args, expected) in [
            (vec!["labelize", "serve"], CompatibilityProfile::Labelary),
            (
                vec!["labelize", "serve", "--default-profile", "labelary"],
                CompatibilityProfile::Labelary,
            ),
            (
                vec![
                    "labelize",
                    "serve",
                    "--default-profile",
                    "zebra-experimental",
                ],
                CompatibilityProfile::ZebraExperimental,
            ),
        ] {
            match Cli::try_parse_from(args).unwrap().command {
                Commands::Serve {
                    default_profile, ..
                } => assert_eq!(default_profile, expected),
                Commands::Convert { .. } => panic!("Expected serve command"),
            }
        }
        assert!(
            Cli::try_parse_from(["labelize", "serve", "--default-profile", "unknown",]).is_err()
        );
    }

    #[test]
    fn experimental_profile_rejects_epl_including_format_override() {
        for format in [
            detect_format(Path::new("label.epl"), None),
            detect_format(Path::new("label.zpl"), Some(InputFormat::Epl)),
        ] {
            assert!(validate_profile(format, CompatibilityProfile::ZebraExperimental).is_err());
            assert!(validate_profile(format, CompatibilityProfile::Labelary).is_ok());
        }
        assert!(validate_profile(
            detect_format(Path::new("label.epl"), Some(InputFormat::Zpl)),
            CompatibilityProfile::ZebraExperimental,
        )
        .is_ok());
    }

    fn test_options() -> DrawerOptions {
        DrawerOptions {
            label_width_mm: 32.0,
            label_height_mm: 32.0,
            dpmm: 8,
            ..Default::default()
        }
    }

    fn png_pixels(data: &[u8]) -> Vec<u8> {
        image::load_from_memory(data).unwrap().to_luma8().into_raw()
    }

    fn pdf_pixels(data: &[u8]) -> Vec<u8> {
        use std::io::Read;

        let doc = lopdf::Document::load_mem(data).unwrap();
        let stream = doc
            .objects
            .values()
            .filter_map(|object| object.as_stream().ok())
            .find(|stream| {
                stream
                    .dict
                    .get(b"Subtype")
                    .and_then(lopdf::Object::as_name)
                    .ok()
                    == Some(b"Image".as_slice())
            })
            .unwrap();
        let mut pixels = Vec::new();
        flate2::read::ZlibDecoder::new(stream.content.as_slice())
            .read_to_end(&mut pixels)
            .unwrap();
        pixels
    }

    #[test]
    fn png_and_raster_pdf_use_the_selected_profile() {
        let labels = parse_labels(EXPLICIT_BYTE_QR, InputFormat::Zpl, 8).unwrap();
        let options = test_options();
        let mut rasters = Vec::new();
        for profile in [
            CompatibilityProfile::Labelary,
            CompatibilityProfile::ZebraExperimental,
        ] {
            let png = render_label(&labels[0], &options, OutputType::Png, profile).unwrap();
            let pdf = render_label(&labels[0], &options, OutputType::Pdf, profile).unwrap();
            let pixels = png_pixels(&png);
            assert_eq!(pdf_pixels(&pdf), pixels);
            rasters.push(pixels);
        }
        assert_ne!(rasters[0], rasters[1]);

        let mut default_png = Cursor::new(Vec::new());
        Renderer::new()
            .draw_label_as_png(&labels[0], &mut default_png, options)
            .unwrap();
        assert_eq!(rasters[0], png_pixels(&default_png.into_inner()));
    }

    #[cfg(feature = "serve")]
    async fn request(
        query: &str,
        content_type: &str,
        body: &'static [u8],
    ) -> axum::response::Response {
        request_with_default(query, content_type, body, CompatibilityProfile::Labelary).await
    }

    #[cfg(feature = "serve")]
    async fn request_with_default(
        query: &str,
        content_type: &str,
        body: &'static [u8],
        default_profile: CompatibilityProfile,
    ) -> axum::response::Response {
        let uri: axum::http::Uri = format!("/convert?{query}").parse().unwrap();
        let params = axum::extract::Query::<ConvertParams>::try_from_uri(&uri).unwrap();
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            axum::http::header::CONTENT_TYPE,
            content_type.parse().unwrap(),
        );
        convert_handler(
            axum::extract::State(default_profile),
            headers,
            params,
            axum::body::Bytes::from_static(body),
        )
        .await
    }

    #[cfg(feature = "serve")]
    #[tokio::test]
    async fn http_rejects_invalid_profiles_and_experimental_epl() {
        for query in ["profile=unknown", "profile=", "profile=Labelary"] {
            let response = request(query, "application/zpl", EXPLICIT_BYTE_QR).await;
            assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
        }
        let response = request("profile=zebra-experimental", "application/epl", b"N\nP1\n").await;
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);

        let response = request(
            "profile=labelary",
            "application/epl",
            b"N\nLO10,10,2,2\nP1\n",
        )
        .await;
        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    #[cfg(feature = "serve")]
    #[tokio::test]
    async fn http_profile_selection_reaches_png_and_pdf() {
        let labels = parse_labels(EXPLICIT_BYTE_QR, InputFormat::Zpl, 8).unwrap();
        for (selection, profile) in [
            ("", CompatibilityProfile::Labelary),
            ("&profile=labelary", CompatibilityProfile::Labelary),
            (
                "&profile=zebra-experimental",
                CompatibilityProfile::ZebraExperimental,
            ),
        ] {
            let expected_png =
                render_label(&labels[0], &test_options(), OutputType::Png, profile).unwrap();
            let expected_pixels = png_pixels(&expected_png);
            for output in ["png", "pdf"] {
                let response = request(
                    &format!("width=32&height=32&dpmm=8&output={output}{selection}"),
                    "application/zpl",
                    EXPLICIT_BYTE_QR,
                )
                .await;
                assert_eq!(response.status(), axum::http::StatusCode::OK);
                let expected_content_type = if output == "png" {
                    "image/png"
                } else {
                    "application/pdf"
                };
                assert_eq!(
                    response.headers()[axum::http::header::CONTENT_TYPE],
                    expected_content_type
                );
                let data = axum::body::to_bytes(response.into_body(), usize::MAX)
                    .await
                    .unwrap();
                let pixels = if output == "png" {
                    png_pixels(&data)
                } else {
                    pdf_pixels(&data)
                };
                assert_eq!(pixels, expected_pixels);
            }
        }
    }

    #[cfg(feature = "serve")]
    #[tokio::test]
    async fn http_uses_server_default_and_allows_request_override() {
        let default_profile = CompatibilityProfile::ZebraExperimental;
        let labels = parse_labels(EXPLICIT_BYTE_QR, InputFormat::Zpl, 8).unwrap();
        for (selection, expected_profile) in [
            ("", CompatibilityProfile::ZebraExperimental),
            ("&profile=labelary", CompatibilityProfile::Labelary),
        ] {
            let expected_png = render_label(
                &labels[0],
                &test_options(),
                OutputType::Png,
                expected_profile,
            )
            .unwrap();
            for output in ["png", "pdf"] {
                let response = request_with_default(
                    &format!("width=32&height=32&dpmm=8&output={output}{selection}"),
                    "application/zpl",
                    EXPLICIT_BYTE_QR,
                    default_profile,
                )
                .await;
                assert_eq!(response.status(), axum::http::StatusCode::OK);
                let data = axum::body::to_bytes(response.into_body(), usize::MAX)
                    .await
                    .unwrap();
                let pixels = if output == "png" {
                    png_pixels(&data)
                } else {
                    pdf_pixels(&data)
                };
                assert_eq!(pixels, png_pixels(&expected_png));
            }
        }
        for query in ["profile=unknown", "profile="] {
            let response =
                request_with_default(query, "application/zpl", EXPLICIT_BYTE_QR, default_profile)
                    .await;
            assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
        }
        for (query, expected_status) in [
            ("", axum::http::StatusCode::BAD_REQUEST),
            (
                "profile=zebra-experimental",
                axum::http::StatusCode::BAD_REQUEST,
            ),
            ("profile=labelary", axum::http::StatusCode::OK),
        ] {
            let response = request_with_default(
                query,
                "application/epl",
                b"N\nLO10,10,2,2\nP1\n",
                default_profile,
            )
            .await;
            assert_eq!(response.status(), expected_status);
        }
    }
}
