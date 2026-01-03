use std::env;
use std::fs::File;
use std::path::Path;

fn main() {
    let png_path = Path::new("assets/icon.png");
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let out_dir = Path::new(&out_dir);

    // Tell Cargo to re-run this build script if the icon file changes
    println!("cargo:rerun-if-changed=assets/icon.png");

    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        let ico_path = out_dir.join("icon.ico");

        if !png_path.exists() {
            println!("cargo:warning=Icon not found at assets/icon.png");
            return;
        }

        if let Err(e) = convert_png_to_ico(png_path, &ico_path) {
            eprintln!("cargo:warning=Failed to convert PNG to ICO: {}", e);
            return;
        }

        if let Ok(icon_path) = ico_path.canonicalize() {
            if let Some(icon_str) = icon_path.to_str() {
                res.set_icon(icon_str);
                if let Err(e) = res.compile() {
                    eprintln!("cargo:warning=Failed to embed Windows icon: {}", e);
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let icns_path = out_dir.join("icon.icns");

        if !png_path.exists() {
            eprintln!("cargo:warning=Icon not found at assets/icon.png");
            return;
        }

        if let Err(e) = convert_png_to_icns(png_path, &icns_path, out_dir) {
            eprintln!("cargo:warning=Failed to convert PNG to ICNS: {}", e);
        } else {
            println!("cargo:warning=Converted PNG to ICNS format");
        }
    }
}

#[cfg(target_os = "windows")]
fn convert_png_to_ico(png_path: &Path, ico_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use ico::{IconDir, IconDirEntry, IconImage, ResourceType};
    use image::{imageops::FilterType, ImageReader};

    let img = ImageReader::open(png_path)?.decode()?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();

    let sizes = vec![16, 32, 48, 256];
    let mut icon_dir = IconDir::new(ResourceType::Icon);

    for &size in &sizes {
        let resized = if width != size || height != size {
            image::imageops::resize(&rgba, size, size, FilterType::Lanczos3)
        } else {
            rgba.clone()
        };

        let icon_image = IconImage::from_rgba_data(size, size, resized.into_raw());
        icon_dir.add_entry(IconDirEntry::encode(&icon_image)?);
    }

    let file = File::create(ico_path)?;
    icon_dir.write(file)?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn convert_png_to_icns(
    png_path: &Path,
    icns_path: &Path,
    out_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    use icns::{IconFamily, IconType, Image};
    use image::{imageops::FilterType, ImageReader};

    let img = ImageReader::open(png_path)?.decode()?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();

    // macOS ICNS requires specific sizes with @2x variants
    let sizes = vec![(16, 32), (32, 64), (128, 256), (256, 512), (512, 1024)];

    let iconset_dir = out_dir.join("icon.iconset");
    std::fs::create_dir_all(&iconset_dir)?;

    for (size, size_2x) in &sizes {
        let resized = if width != *size || height != *size {
            image::imageops::resize(&rgba, *size, *size, FilterType::Lanczos3)
        } else {
            rgba.clone()
        };
        let png_path_1x = iconset_dir.join(format!("icon_{}x{}.png", size, size));
        resized.save(&png_path_1x)?;

        let resized_2x = if width != *size_2x || height != *size_2x {
            image::imageops::resize(&rgba, *size_2x, *size_2x, FilterType::Lanczos3)
        } else {
            rgba.clone()
        };
        let png_path_2x = iconset_dir.join(format!("icon_{}x{}@2x.png", size, size));
        resized_2x.save(&png_path_2x)?;
    }

    let mut icon_family = IconFamily::new();

    for (size, size_2x) in &sizes {
        let img_path = iconset_dir.join(format!("icon_{}x{}.png", size, size));
        if let Ok(img) = Image::read_png(File::open(&img_path)?) {
            let icon_type = match size {
                16 => IconType::RGB24_16x16,
                32 => IconType::RGB24_32x32,
                128 => IconType::RGB24_128x128,
                256 => IconType::RGBA32_256x256,
                512 => IconType::RGBA32_512x512,
                _ => continue,
            };
            if icon_family.add_icon_with_type(&img, icon_type).is_err() {
                continue;
            }
        }

        let img_path_2x = iconset_dir.join(format!("icon_{}x{}@2x.png", size, size));
        if let Ok(img) = Image::read_png(File::open(&img_path_2x)?) {
            let icon_type_2x = match size_2x {
                32 => IconType::RGBA32_16x16_2x,
                64 => IconType::RGBA32_32x32_2x,
                256 => IconType::RGBA32_128x128_2x,
                512 => IconType::RGBA32_256x256_2x,
                1024 => IconType::RGBA32_512x512_2x,
                _ => continue,
            };
            if icon_family.add_icon_with_type(&img, icon_type_2x).is_err() {
                continue;
            }
        }
    }

    let file = File::create(icns_path)?;
    icon_family.write(file)?;

    std::fs::remove_dir_all(iconset_dir)?;

    Ok(())
}
