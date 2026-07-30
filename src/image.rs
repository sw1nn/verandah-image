//! Image manipulation utilities.
//!
//! Provides functions for image effects and format conversions.

use std::time::SystemTime;

use image::{Rgb, RgbImage, Rgba, RgbaImage};

/// Apply a slow brightness pulse to the image based on system time.
///
/// Creates a sine wave oscillation between 10% and 100% brightness
/// with a 1.5 second cycle. Useful for attention-grabbing animations.
pub fn apply_brightness_pulse(rgba: &mut RgbaImage) {
    // Use subsec portion for precision (f32 can't handle billions of seconds)
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    // Use seconds mod 3 + subsec (3 is a multiple of 1.5, so sine wave aligns at wrap)
    let secs = (now.as_secs() % 3) as f32 + now.subsec_nanos() as f32 / 1_000_000_000.0;

    // Sine wave oscillating between 0.1 and 1.0 (dims to 10% at darkest)
    // Divide by 1.5 for one cycle per 1.5 seconds
    let pulse = (secs * std::f32::consts::TAU / 1.5).sin() * 0.45 + 0.55;

    tracing::debug!(pulse, "apply_brightness_pulse");

    for pixel in rgba.pixels_mut() {
        pixel[0] = (pixel[0] as f32 * pulse) as u8;
        pixel[1] = (pixel[1] as f32 * pulse) as u8;
        pixel[2] = (pixel[2] as f32 * pulse) as u8;
    }
}

/// Convert RGB to greyscale using the luminosity method.
///
/// Uses standard luminosity coefficients: 0.299*R + 0.587*G + 0.114*B
#[inline]
pub fn to_greyscale(r: u8, g: u8, b: u8) -> u8 {
    ((0.299 * r as f32) + (0.587 * g as f32) + (0.114 * b as f32)) as u8
}

/// Convert an RgbImage to an RgbaImage with full opacity.
pub fn rgb_to_rgba(rgb: &RgbImage) -> RgbaImage {
    RgbaImage::from_fn(rgb.width(), rgb.height(), |x, y| {
        let pixel = rgb.get_pixel(x, y);
        Rgba([pixel[0], pixel[1], pixel[2], 255])
    })
}

/// Convert an RgbaImage to an RgbImage, discarding alpha.
pub fn rgba_to_rgb(rgba: &RgbaImage) -> RgbImage {
    RgbImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        let pixel = rgba.get_pixel(x, y);
        Rgb([pixel[0], pixel[1], pixel[2]])
    })
}

/// Convert raw RGB bytes to an RgbImage.
///
/// # Arguments
/// * `width` - Image width
/// * `height` - Image height
/// * `data` - Raw RGB bytes (length must be width * height * 3)
pub fn bytes_to_rgb(width: u32, height: u32, data: &[u8]) -> RgbImage {
    RgbImage::from_fn(width, height, |x, y| {
        let idx = ((y * width + x) * 3) as usize;
        if idx + 2 < data.len() {
            Rgb([data[idx], data[idx + 1], data[idx + 2]])
        } else {
            Rgb([0, 0, 0])
        }
    })
}

/// Convert raw RGB bytes to an RgbaImage with full opacity.
///
/// # Arguments
/// * `width` - Image width
/// * `height` - Image height
/// * `data` - Raw RGB bytes (length must be width * height * 3)
pub fn bytes_to_rgba(width: u32, height: u32, data: &[u8]) -> RgbaImage {
    RgbaImage::from_fn(width, height, |x, y| {
        let idx = ((y * width + x) * 3) as usize;
        if idx + 2 < data.len() {
            Rgba([data[idx], data[idx + 1], data[idx + 2], 255])
        } else {
            Rgba([0, 0, 0, 255])
        }
    })
}

/// Scale an image to fit within target dimensions using high-quality Lanczos3 filter.
pub fn scale_image(src: &RgbImage, target_width: u32, target_height: u32) -> RgbImage {
    if src.width() == target_width && src.height() == target_height {
        return src.clone();
    }

    image::imageops::resize(
        src,
        target_width,
        target_height,
        image::imageops::FilterType::Lanczos3,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_greyscale_black() {
        assert_eq!(to_greyscale(0, 0, 0), 0);
    }

    #[test]
    fn test_to_greyscale_white() {
        assert_eq!(to_greyscale(255, 255, 255), 255);
    }

    #[test]
    fn test_to_greyscale_red() {
        // 0.299 * 255 ≈ 76
        let grey = to_greyscale(255, 0, 0);
        assert!(grey > 70 && grey < 80);
    }

    #[test]
    fn test_to_greyscale_green() {
        // 0.587 * 255 ≈ 150
        let grey = to_greyscale(0, 255, 0);
        assert!(grey > 145 && grey < 155);
    }

    #[test]
    fn test_rgb_to_rgba_preserves_colors() {
        let rgb = RgbImage::from_pixel(2, 2, Rgb([100, 150, 200]));
        let rgba = rgb_to_rgba(&rgb);

        assert_eq!(rgba.width(), 2);
        assert_eq!(rgba.height(), 2);
        assert_eq!(*rgba.get_pixel(0, 0), Rgba([100, 150, 200, 255]));
    }

    #[test]
    fn test_rgba_to_rgb_discards_alpha() {
        let rgba = RgbaImage::from_pixel(2, 2, Rgba([100, 150, 200, 128]));
        let rgb = rgba_to_rgb(&rgba);

        assert_eq!(rgb.width(), 2);
        assert_eq!(rgb.height(), 2);
        assert_eq!(*rgb.get_pixel(0, 0), Rgb([100, 150, 200]));
    }

    #[test]
    fn test_bytes_to_rgb() {
        let data = vec![255, 0, 0, 0, 255, 0, 0, 0, 255, 128, 128, 128];
        let img = bytes_to_rgb(2, 2, &data);

        assert_eq!(*img.get_pixel(0, 0), Rgb([255, 0, 0]));
        assert_eq!(*img.get_pixel(1, 0), Rgb([0, 255, 0]));
        assert_eq!(*img.get_pixel(0, 1), Rgb([0, 0, 255]));
        assert_eq!(*img.get_pixel(1, 1), Rgb([128, 128, 128]));
    }

    #[test]
    fn test_scale_image_same_size_returns_clone() {
        let img = RgbImage::from_pixel(10, 10, Rgb([100, 100, 100]));
        let scaled = scale_image(&img, 10, 10);
        assert_eq!(scaled.dimensions(), (10, 10));
    }

    #[test]
    fn test_scale_image_resizes() {
        let img = RgbImage::from_pixel(10, 10, Rgb([100, 100, 100]));
        let scaled = scale_image(&img, 20, 20);
        assert_eq!(scaled.dimensions(), (20, 20));
    }
}
