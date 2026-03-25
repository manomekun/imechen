use image::ImageFormat;
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::path::{Path, PathBuf};

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

#[derive(Debug, Serialize)]
pub struct ImageInfo {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub thumbnail: String,
}

fn generate_thumbnail(img: &image::DynamicImage) -> String {
    use base64::Engine;
    let thumb = img.thumbnail(72, 72);
    let mut buf = Cursor::new(Vec::new());
    thumb.write_to(&mut buf, ImageFormat::Png).unwrap_or_default();
    let b64 = base64::engine::general_purpose::STANDARD.encode(buf.into_inner());
    format!("data:image/png;base64,{}", b64)
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

#[tauri::command]
pub async fn get_image_info(path: String) -> Result<ImageInfo, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let file_path = Path::new(&path);
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

        let thumbnail = generate_thumbnail(&img);

        Ok(ImageInfo {
            path,
            name,
            size: metadata.len(),
            width: img.width(),
            height: img.height(),
            format,
            thumbnail,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Compress PNG using pngquant (imagequant) for lossy quantization + oxipng for lossless optimization.
/// Quality range: 0 (smallest, worst) to 100 (largest, best).
fn compress_png(
    img: &image::DynamicImage,
    output_path: &Path,
    quality: u8,
) -> Result<(), String> {
    let rgba = img.to_rgba8();
    let width = rgba.width() as usize;
    let height = rgba.height() as usize;
    let raw_pixels = rgba.as_raw();

    // Convert &[u8] (RGBA bytes) to &[imagequant::RGBA] for imagequant
    let pixels: &[imagequant::RGBA] = unsafe {
        std::slice::from_raw_parts(raw_pixels.as_ptr() as *const imagequant::RGBA, width * height)
    };

    // Step 1: imagequant (pngquant) - lossy color quantization
    let mut liq = imagequant::new();
    let min_quality = quality.saturating_sub(20);
    liq.set_quality(min_quality, quality).map_err(|e| e.to_string())?;

    let mut liq_image = liq
        .new_image_borrowed(pixels, width, height, 0.0)
        .map_err(|e| e.to_string())?;

    let mut result = liq.quantize(&mut liq_image).map_err(|e| e.to_string())?;
    result.set_dithering_level(1.0).map_err(|e| e.to_string())?;
    let (palette, quantized_pixels) = result.remapped(&mut liq_image).map_err(|e| e.to_string())?;

    // Step 2: Encode as indexed PNG using image crate
    // Build an RGBA image from quantized palette + indices
    let mut output_rgba = Vec::with_capacity(width * height * 4);
    for &idx in &quantized_pixels {
        let c = &palette[idx as usize];
        output_rgba.extend_from_slice(&[c.r, c.g, c.b, c.a]);
    }

    let quantized_img =
        image::RgbaImage::from_raw(width as u32, height as u32, output_rgba)
            .ok_or("Failed to create quantized image")?;

    let mut png_buf = Cursor::new(Vec::new());
    quantized_img
        .write_to(&mut png_buf, ImageFormat::Png)
        .map_err(|e| e.to_string())?;

    // Step 3: oxipng - lossless re-compression
    let optimized = oxipng::optimize_from_memory(
        png_buf.get_ref(),
        &oxipng::Options::from_preset(2),
    )
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

#[tauri::command]
pub async fn convert_image(request: ConvertImageRequest) -> Result<ConvertImageResult, String> {
    tauri::async_runtime::spawn_blocking(move || convert_image_sync(request))
        .await
        .map_err(|e| e.to_string())?
}

fn convert_image_sync(request: ConvertImageRequest) -> Result<ConvertImageResult, String> {
    let source = Path::new(&request.source_path);
    let original_size = std::fs::metadata(source).map_err(|e| e.to_string())?.len();
    let mut img = image::open(source).map_err(|e| e.to_string())?;

    // Resize if requested
    if let (Some(w), Some(h)) = (request.resize_width, request.resize_height) {
        let keep_aspect = request.keep_aspect_ratio.unwrap_or(true);
        if keep_aspect {
            img = img.resize(w, h, image::imageops::FilterType::Lanczos3);
        } else {
            img = img.resize_exact(w, h, image::imageops::FilterType::Lanczos3);
        }
    }

    let target_format = parse_format(&request.output_format)?;
    let ext = format_extension(target_format);

    let stem = source
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();
    let output_path: PathBuf = Path::new(&request.output_dir).join(format!("{}.{}", stem, ext));

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

    Ok(ConvertImageResult {
        output_path: output_path.to_string_lossy().to_string(),
        original_size,
        output_size,
        width: img.width(),
        height: img.height(),
    })
}
