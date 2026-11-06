use crate::output::OutputReservation;
use image::ImageFormat;
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct ConvertImageRequest {
    pub source_path: String,
    pub output_format: String,
    pub output_dir: String,
    pub quality: Option<u8>,
    pub resize_width: Option<u32>,
    pub resize_height: Option<u32>,
    pub keep_aspect_ratio: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct ConvertImageResult {
    pub output_path: String,
    pub original_size: u64,
    pub output_size: u64,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug)]
pub struct ImageInfo {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub thumbnail_png: Vec<u8>,
}

/// Frontends show thumbnails in a 110px-wide cell, so this covers 2x displays
/// without them having to decode full-size sources once per row.
pub const THUMBNAIL_MAX: u32 = 220;

fn generate_thumbnail(img: &image::DynamicImage) -> Vec<u8> {
    let thumb = img.thumbnail(THUMBNAIL_MAX, THUMBNAIL_MAX);
    let mut buf = Cursor::new(Vec::new());
    thumb
        .write_to(&mut buf, ImageFormat::Png)
        .unwrap_or_default();
    buf.into_inner()
}

fn parse_format(format: &str) -> Result<ImageFormat, String> {
    match format.to_lowercase().as_str() {
        "jpeg" | "jpg" => Ok(ImageFormat::Jpeg),
        "png" => Ok(ImageFormat::Png),
        "gif" => Ok(ImageFormat::Gif),
        "webp" => Ok(ImageFormat::WebP),
        "bmp" => Ok(ImageFormat::Bmp),
        _ => Err(format!("Unsupported format: {}", format)),
    }
}

fn format_extension(format: ImageFormat) -> &'static str {
    match format {
        ImageFormat::Jpeg => "jpg",
        ImageFormat::Png => "png",
        ImageFormat::Gif => "gif",
        ImageFormat::WebP => "webp",
        ImageFormat::Bmp => "bmp",
        _ => "bin",
    }
}

pub fn get_image_info(path: &str) -> Result<ImageInfo, String> {
    let file_path = Path::new(path);
    let metadata = std::fs::metadata(file_path).map_err(|e| e.to_string())?;
    let img = image::open(file_path).map_err(|e| e.to_string())?;

    let name = file_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let format = image::ImageFormat::from_path(file_path)
        .map(|f| format!("{:?}", f).to_lowercase())
        .unwrap_or_else(|_| "unknown".to_string());

    Ok(ImageInfo {
        path: path.to_string(),
        name,
        size: metadata.len(),
        width: img.width(),
        height: img.height(),
        format,
        thumbnail_png: generate_thumbnail(&img),
    })
}

/// Compress PNG using pngquant (imagequant) for lossy quantization + oxipng for lossless optimization.
/// Quality range: 0 (smallest, worst) to 100 (largest, best).
/// When quality is 100, the lossy pngquant step is skipped and only the lossless oxipng pass runs.
fn compress_png(img: &image::DynamicImage, output_path: &Path, quality: u8) -> Result<(), String> {
    let rgba = img.to_rgba8();
    let width = rgba.width();
    let height = rgba.height();

    let png_buf = if quality >= 100 {
        // Lossless path: encode the source pixels straight to PNG and let oxipng handle the rest.
        let mut buf = Cursor::new(Vec::new());
        rgba.write_to(&mut buf, ImageFormat::Png)
            .map_err(|e| e.to_string())?;
        buf.into_inner()
    } else {
        let width = width as usize;
        let height = height as usize;
        let raw_pixels = rgba.as_raw();

        // Convert &[u8] (RGBA bytes) to &[imagequant::RGBA] for imagequant
        let pixels: &[imagequant::RGBA] = unsafe {
            std::slice::from_raw_parts(
                raw_pixels.as_ptr() as *const imagequant::RGBA,
                width * height,
            )
        };

        // imagequant (pngquant) - lossy color quantization
        let mut liq = imagequant::new();
        let min_quality = quality.saturating_sub(20);
        liq.set_quality(min_quality, quality)
            .map_err(|e| e.to_string())?;

        let mut liq_image = liq
            .new_image_borrowed(pixels, width, height, 0.0)
            .map_err(|e| e.to_string())?;

        let mut result = liq.quantize(&mut liq_image).map_err(|e| e.to_string())?;
        result.set_dithering_level(1.0).map_err(|e| e.to_string())?;
        let (palette, quantized_pixels) =
            result.remapped(&mut liq_image).map_err(|e| e.to_string())?;

        // Encode quantized palette + indices as an RGBA PNG
        let mut output_rgba = Vec::with_capacity(width * height * 4);
        for &idx in &quantized_pixels {
            let c = &palette[idx as usize];
            output_rgba.extend_from_slice(&[c.r, c.g, c.b, c.a]);
        }

        let quantized_img = image::RgbaImage::from_raw(width as u32, height as u32, output_rgba)
            .ok_or("Failed to create quantized image")?;

        let mut buf = Cursor::new(Vec::new());
        quantized_img
            .write_to(&mut buf, ImageFormat::Png)
            .map_err(|e| e.to_string())?;
        buf.into_inner()
    };

    // oxipng - lossless re-compression (always applied)
    let optimized = oxipng::optimize_from_memory(&png_buf, &oxipng::Options::from_preset(2))
        .map_err(|e| e.to_string())?;

    std::fs::write(output_path, optimized).map_err(|e| e.to_string())?;
    Ok(())
}

/// Compress WebP using the webp crate with quality parameter (0.0 - 100.0).
fn compress_webp(
    img: &image::DynamicImage,
    output_path: &Path,
    quality: f32,
) -> Result<(), String> {
    let rgba = img.to_rgba8();
    let width = rgba.width();
    let height = rgba.height();

    let encoder = webp::Encoder::from_rgba(rgba.as_raw(), width, height);
    let webp_data = encoder.encode(quality);

    std::fs::write(output_path, &*webp_data).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn convert_image(request: ConvertImageRequest) -> Result<ConvertImageResult, String> {
    let source = Path::new(&request.source_path);
    let original_size = std::fs::metadata(source).map_err(|e| e.to_string())?.len();
    let mut img = image::open(source).map_err(|e| e.to_string())?;

    // Resize if requested.
    // Cases:
    //   - both width & height: keep_aspect=true → fit within bounds; false → exact stretch
    //   - width only:  keep_aspect=true → height auto from source ratio; false → height unchanged
    //   - height only: keep_aspect=true → width auto from source ratio;  false → width unchanged
    let keep_aspect = request.keep_aspect_ratio.unwrap_or(true);
    let filter = image::imageops::FilterType::Lanczos3;
    let (src_w, src_h) = (img.width(), img.height());
    img = match (request.resize_width, request.resize_height) {
        (Some(w), Some(h)) => {
            if keep_aspect {
                img.resize(w, h, filter)
            } else {
                img.resize_exact(w, h, filter)
            }
        }
        (Some(w), None) => {
            let h = if keep_aspect && src_w > 0 {
                ((w as u64 * src_h as u64) / src_w as u64).max(1) as u32
            } else {
                src_h
            };
            img.resize_exact(w, h, filter)
        }
        (None, Some(h)) => {
            let w = if keep_aspect && src_h > 0 {
                ((h as u64 * src_w as u64) / src_h as u64).max(1) as u32
            } else {
                src_w
            };
            img.resize_exact(w, h, filter)
        }
        (None, None) => img,
    };

    let target_format = parse_format(&request.output_format)?;
    let ext = format_extension(target_format);

    let stem = source.file_stem().unwrap_or_default().to_string_lossy();
    let reservation = OutputReservation::reserve(Path::new(&request.output_dir), &stem, ext)?;
    let output_path = reservation.path().to_path_buf();

    match target_format {
        ImageFormat::Jpeg => {
            let quality = request.quality.unwrap_or(85);
            let file = std::fs::File::create(&output_path).map_err(|e| e.to_string())?;
            let mut writer = std::io::BufWriter::new(file);
            let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut writer, quality);
            img.write_with_encoder(encoder).map_err(|e| e.to_string())?;
        }
        ImageFormat::Png => {
            let quality = request.quality.unwrap_or(80);
            compress_png(&img, &output_path, quality)?;
        }
        ImageFormat::WebP => {
            let quality = request.quality.unwrap_or(80) as f32;
            compress_webp(&img, &output_path, quality)?;
        }
        _ => {
            let file = std::fs::File::create(&output_path).map_err(|e| e.to_string())?;
            let mut writer = std::io::BufWriter::new(file);
            img.write_to(&mut writer, target_format)
                .map_err(|e| e.to_string())?;
        }
    }

    let output_size = std::fs::metadata(&output_path)
        .map_err(|e| e.to_string())?
        .len();
    let output_path = reservation.commit();

    Ok(ConvertImageResult {
        output_path: output_path.to_string_lossy().to_string(),
        original_size,
        output_size,
        width: img.width(),
        height: img.height(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "imechen_image_test_{}_{}",
                tag,
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn write_source(dir: &TempDir, name: &str, w: u32, h: u32) -> String {
        let path = dir.path().join(name);
        image::RgbaImage::from_fn(w, h, |x, y| {
            image::Rgba([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8, 255])
        })
        .save(&path)
        .unwrap();
        path.to_string_lossy().to_string()
    }

    fn request(source: &str, dir: &TempDir, format: &str) -> ConvertImageRequest {
        ConvertImageRequest {
            source_path: source.to_string(),
            output_format: format.to_string(),
            output_dir: dir.path().join("out").to_string_lossy().to_string(),
            quality: None,
            resize_width: None,
            resize_height: None,
            keep_aspect_ratio: None,
        }
    }

    fn convert(tag: &str, width: Option<u32>, height: Option<u32>, keep: bool) -> (u32, u32) {
        let dir = TempDir::new(tag);
        std::fs::create_dir_all(dir.path().join("out")).unwrap();
        let source = write_source(&dir, "src.png", 200, 100);
        let mut req = request(&source, &dir, "png");
        req.resize_width = width;
        req.resize_height = height;
        req.keep_aspect_ratio = Some(keep);

        let result = convert_image(req).expect("convert_image failed");

        let decoded = image::open(&result.output_path).unwrap();
        assert_eq!(
            (decoded.width(), decoded.height()),
            (result.width, result.height)
        );
        (result.width, result.height)
    }

    #[test]
    fn width_and_height_with_aspect_ratio_fit_within_bounds_preserving_ratio() {
        assert_eq!(convert("fit", Some(50), Some(50), true), (50, 25));
    }

    #[test]
    fn width_and_height_without_aspect_ratio_stretch_to_exact_size() {
        assert_eq!(convert("exact", Some(50), Some(50), false), (50, 50));
    }

    #[test]
    fn width_only_with_aspect_ratio_derives_height_from_source_ratio() {
        assert_eq!(convert("w_keep", Some(100), None, true), (100, 50));
    }

    #[test]
    fn width_only_without_aspect_ratio_keeps_source_height() {
        assert_eq!(convert("w_free", Some(100), None, false), (100, 100));
    }

    #[test]
    fn height_only_with_aspect_ratio_derives_width_from_source_ratio() {
        assert_eq!(convert("h_keep", None, Some(50), true), (100, 50));
    }

    #[test]
    fn height_only_without_aspect_ratio_keeps_source_width() {
        assert_eq!(convert("h_free", None, Some(50), false), (200, 50));
    }

    #[test]
    fn no_resize_keeps_source_dimensions() {
        assert_eq!(convert("none", None, None, true), (200, 100));
    }

    #[test]
    fn output_is_named_after_source_stem_with_target_extension() {
        let dir = TempDir::new("naming");
        std::fs::create_dir_all(dir.path().join("out")).unwrap();
        let source = write_source(&dir, "photo.png", 16, 16);

        for (format, ext) in [
            ("jpeg", "jpg"),
            ("jpg", "jpg"),
            ("PNG", "png"),
            ("webp", "webp"),
        ] {
            let mut req = request(&source, &dir, format);
            req.output_dir = dir
                .path()
                .join(format!("out_{}", format))
                .to_string_lossy()
                .to_string();
            std::fs::create_dir_all(&req.output_dir).unwrap();
            let expected = Path::new(&req.output_dir).join(format!("photo.{}", ext));

            let result = convert_image(req).expect("convert_image failed");

            assert_eq!(
                Path::new(&result.output_path),
                expected,
                "format {}",
                format
            );
            assert!(expected.exists());
        }
    }

    #[test]
    fn unsupported_output_format_is_rejected() {
        let dir = TempDir::new("unsupported");
        std::fs::create_dir_all(dir.path().join("out")).unwrap();
        let source = write_source(&dir, "photo.png", 16, 16);

        let err = convert_image(request(&source, &dir, "tiff")).unwrap_err();

        assert!(err.contains("Unsupported format"), "{}", err);
    }

    #[test]
    fn png_output_is_decodable_at_lossless_and_lossy_quality() {
        for quality in [100u8, 50] {
            let dir = TempDir::new(&format!("png_q{}", quality));
            std::fs::create_dir_all(dir.path().join("out")).unwrap();
            let source = write_source(&dir, "src.png", 64, 48);
            let mut req = request(&source, &dir, "png");
            req.quality = Some(quality);

            let result = convert_image(req).expect("convert_image failed");

            let decoded = image::open(&result.output_path).expect("output is not decodable");
            assert_eq!(
                image::ImageFormat::from_path(&result.output_path).unwrap(),
                ImageFormat::Png
            );
            assert_eq!(
                (decoded.width(), decoded.height()),
                (64, 48),
                "quality {}",
                quality
            );
        }
    }

    #[test]
    fn get_image_info_reports_metadata_and_a_small_png_thumbnail() {
        let dir = TempDir::new("info");
        let source = write_source(&dir, "wide.png", 300, 150);

        let info = get_image_info(&source).expect("get_image_info failed");

        assert_eq!(info.path, source);
        assert_eq!(info.name, "wide.png");
        assert_eq!(info.size, std::fs::metadata(&source).unwrap().len());
        assert_eq!((info.width, info.height), (300, 150));
        assert_eq!(info.format, "png");
        let thumb = image::load_from_memory_with_format(&info.thumbnail_png, ImageFormat::Png)
            .expect("thumbnail is not a PNG");
        assert!(thumb.width() <= THUMBNAIL_MAX && thumb.height() <= THUMBNAIL_MAX);
    }

    fn same_dir_request(source: &str, dir: &TempDir, format: &str) -> ConvertImageRequest {
        let mut req = request(source, dir, format);
        req.output_dir = dir.path().to_string_lossy().to_string();
        req
    }

    #[test]
    fn existing_output_is_not_overwritten_and_gets_a_number() {
        let dir = TempDir::new("exists");
        let source = write_source(&dir, "photo.png", 16, 16);
        std::fs::write(dir.path().join("photo.webp"), b"keep me").unwrap();

        let first = convert_image(same_dir_request(&source, &dir, "webp")).unwrap();
        let second = convert_image(same_dir_request(&source, &dir, "webp")).unwrap();

        assert_eq!(
            Path::new(&first.output_path),
            dir.path().join("photo(1).webp")
        );
        assert_eq!(
            Path::new(&second.output_path),
            dir.path().join("photo(2).webp")
        );
        assert_eq!(
            std::fs::read(dir.path().join("photo.webp")).unwrap(),
            b"keep me"
        );
    }

    #[test]
    fn same_stem_inputs_in_one_batch_do_not_overwrite_each_other() {
        let dir = TempDir::new("batch");
        let jpg = dir.path().join("a.jpg").to_string_lossy().to_string();
        image::RgbImage::from_pixel(16, 16, image::Rgb([200, 10, 10]))
            .save(&jpg)
            .unwrap();
        let png = write_source(&dir, "a.png", 24, 24);
        std::fs::create_dir_all(dir.path().join("out")).unwrap();

        let first = convert_image(request(&jpg, &dir, "webp")).unwrap();
        let second = convert_image(request(&png, &dir, "webp")).unwrap();

        let out = dir.path().join("out");
        assert_eq!(Path::new(&first.output_path), out.join("a.webp"));
        assert_eq!(Path::new(&second.output_path), out.join("a(1).webp"));
        assert_eq!(image::open(&first.output_path).unwrap().width(), 16);
        assert_eq!(image::open(&second.output_path).unwrap().width(), 24);
    }

    #[test]
    fn converting_in_place_to_the_same_format_keeps_the_source_intact() {
        let dir = TempDir::new("inplace");
        let source = write_source(&dir, "photo.png", 32, 32);
        let before = std::fs::read(&source).unwrap();

        let result = convert_image(same_dir_request(&source, &dir, "png")).unwrap();

        assert_eq!(
            Path::new(&result.output_path),
            dir.path().join("photo(1).png")
        );
        assert_eq!(std::fs::read(&source).unwrap(), before);
        assert!(image::open(&result.output_path).is_ok());
    }

    #[test]
    fn dotted_source_name_keeps_its_dots_before_the_number() {
        let dir = TempDir::new("dotted");
        let source = write_source(&dir, "my.photo.png", 16, 16);
        std::fs::write(dir.path().join("my.photo.webp"), b"x").unwrap();

        let result = convert_image(same_dir_request(&source, &dir, "webp")).unwrap();

        assert_eq!(
            Path::new(&result.output_path),
            dir.path().join("my.photo(1).webp")
        );
    }

    #[test]
    fn failed_conversion_leaves_no_reserved_file() {
        let dir = TempDir::new("failed");
        let source = write_source(&dir, "photo.png", 16, 16);
        std::fs::create_dir_all(dir.path().join("out")).unwrap();
        // JPEG is limited to 65535 px per side, so the encoder fails only
        // after the output name has been reserved.
        let mut req = request(&source, &dir, "jpeg");
        req.resize_width = Some(70_000);
        req.keep_aspect_ratio = Some(false);

        let result = convert_image(req);

        assert!(result.is_err());
        assert_eq!(
            std::fs::read_dir(dir.path().join("out")).unwrap().count(),
            0
        );
    }
}
