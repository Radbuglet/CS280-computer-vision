mod util;

use crate::util::error::{AnyResult, ErrorFormatExt};
use crate::util::pixel::{lin_remap, px_img_to_pal, px_pal_to_img, DecomposablePixel};
use anyhow::Context;
use image::{ImageBuffer, RgbaImage};
use num_traits::Num;
use palette::rgb::Rgba;
use palette::{Hsva, Hue, IntoColor, Laba, LinSrgba, RgbHue, Srgb};

fn main() {
    if let Err(err) = main_fallible() {
        eprintln!("{}", err.format_error());
        std::process::exit(1);
    }
    println!("Finished!");
}

fn main_fallible() -> AnyResult<()> {
    println!(
        "Current working directory: {}",
        std::env::current_dir().map_or("<NO CWD FOUND>".to_string(), |path| path
            .to_string_lossy()
            .into_owned()
            .to_string())
    );

    // Load images
    let image1 = image::open("images/in/image1.jpg")
        .context("Could not open 'images/in/color-monke.jpg'. Is the CWD correct?")?
        .to_rgba8();

    let image2 = image::open("images/in/image2.jpg")
        .context("Could not open 'images/in/color-monke.jpg'. Is the CWD correct?")?
        .to_rgba8();

    // Darken
    map_image(&image1, |pixel, _, _| {
        // We could also use the built-in darken function but that feels like cheating...
        LinSrgba::compose(pixel.decompose().iter().copied().map(|comp| comp * 0.5))
    })
    .save("images/image1_dark.jpg")?;

    // Make grayscale
    map_image(&image1, |pixel, _, _| {
        // We could also convert it into luma and back but again... cheating.
        let luma = (pixel.red + pixel.green + pixel.blue) / 3.;
        LinSrgba::new(luma, luma, luma, pixel.alpha)
    })
    .save("images/image1_grayscale.jpg")?;

    // RGB component masking
    const PRESERVE: Option<f32> = None;
    const ZERO: Option<f32> = Some(0.);
    const ONE: Option<f32> = Some(1.);

    map_image(&image2, xform_rgba_mask(&[PRESERVE, ZERO, ZERO, PRESERVE]))
        .save("images/image2_only_red.jpg")?;

    map_image(&image2, xform_rgba_mask(&[ZERO, PRESERVE, ZERO, PRESERVE]))
        .save("images/image2_only_green.jpg")?;

    map_image(&image2, xform_rgba_mask(&[ZERO, ZERO, PRESERVE, PRESERVE]))
        .save("images/image2_only_blue.jpg")?;

    // LAB component masking
    map_image(&image1, xform_laba_mask(&[PRESERVE, ZERO, ZERO, PRESERVE]))
        .save("images/image1_only_l.jpg")?;

    map_image(&image1, xform_laba_mask(&[PRESERVE, PRESERVE, ZERO, PRESERVE]))
        .save("images/image1_only_la.jpg")?;

    map_image(&image1, xform_laba_mask(&[PRESERVE, ZERO, PRESERVE, PRESERVE]))
        .save("images/image1_only_lb.jpg")?;

    // HSV component masking
    map_image(&image1, xform_hsva_mask(&[PRESERVE, PRESERVE, PRESERVE, PRESERVE]))
        .save("images/image1_hsv_debug.jpg")?;

    map_image(&image1, xform_hsva_mask(&[ZERO, ZERO, PRESERVE, PRESERVE]))
        .save("images/image1_only_v.jpg")?;

    map_image(&image1, xform_hsva_mask(&[PRESERVE, ZERO, PRESERVE, PRESERVE]))
        .save("images/image1_only_hv.jpg")?;

    map_image(&image1, xform_hsva_mask(&[PRESERVE, ONE, PRESERVE, PRESERVE]))
        .save("images/image1_full_saturation.jpg")?;

    map_image(&image2, xform_hsva_mask(&[PRESERVE, ONE, PRESERVE, PRESERVE]))
        .save("images/image2_full_saturation.jpg")?;

    map_image(&image1, xform_hsva_mask(&[ZERO, PRESERVE, PRESERVE, PRESERVE]))
        .save("images/image1_only_sv.jpg")?;

    // HSV hue manipulation
    map_image(&image1, |pixel, x, _| {
        // I sincerely have no idea why palette is forcing me to convert to Rgba as an intermediary.
        let pixel: Rgba = pixel.into_color();
        let pixel: Hsva = pixel.into_color();

        // Cycle through the entire hue shift spectrum
        let pixel = pixel.shift_hue(lin_remap(x, 0..image1.width(), 0.0..360.0));

        // Do the conversion backwards
        let pixel: Rgba = pixel.into_color();
        pixel.into_color()
    })
    .save("images/image_1_hue_shift.jpg")?;

    map_image(&image1, |pixel, x, y| {
        // Convert to HSVa - TODO: make a utility function for this once I figure out what's going on.
        let pixel: Rgba = pixel.into_color();
        let mut pixel: Hsva = pixel.into_color();

        // Cycle through the entire hue shift spectrum
        pixel.hue = RgbHue::from_degrees(lin_remap(x, 0..image1.width(), 0.0..360.0));
        pixel.saturation = lin_remap(y, 0..image1.height(), 0.0..1.0);

        // Do the conversion backwards
        let pixel: Rgba = pixel.into_color();
        pixel.into_color()
    })
    .save("images/image_1_hue_set.jpg")?;

    // Combining multiple images
    map_image(&image1, |pixel, x, y| {
        let x_side = x > image1.width() / 2;
        let y_side = y > image1.height() / 2;

        match (x_side, y_side) {
            (false, false) => xform_rgba_mask(&[PRESERVE, ZERO, ZERO, PRESERVE])(pixel, x, y),
            (true, false) => xform_rgba_mask(&[ZERO, PRESERVE, ZERO, PRESERVE])(pixel, x, y),
            (false, true) => xform_rgba_mask(&[ZERO, ZERO, PRESERVE, PRESERVE])(pixel, x, y),
            (true, true) => pixel,
        }
    })
    .save("images/image1_combined.jpg")?;

    // Very bad mosaic effect
    // We save this as a png because jpg leaves very visible artifacts.
    ImageBuffer::from_fn(image1.width(), image1.height(), |x, y| {
        let grain = 10;
        *image1.get_pixel(x / grain * grain, y / grain * grain)
    })
    .save("images/image1_mosaic.png")?;

    Ok(())
}

// === Image utils === //

fn map_image<F>(image: &RgbaImage, mut map: F) -> RgbaImage
where
    F: FnMut(LinSrgba, u32, u32) -> LinSrgba,
{
    ImageBuffer::from_fn(image.width(), image.height(), move |x, y| {
        let pixel = px_img_to_pal(*image.get_pixel(x, y));
        px_pal_to_img(map(pixel, x, y))
    })
}

fn xform_rgba_mask<'a>(mask: &'a [Option<f32>]) -> impl 'a + FnMut(LinSrgba, u32, u32) -> LinSrgba {
    move |pixel, _, _| LinSrgba::compose(vec_mask(pixel.decompose(), mask))
}

fn xform_laba_mask<'a>(mask: &'a [Option<f32>]) -> impl 'a + FnMut(LinSrgba, u32, u32) -> LinSrgba {
    move |pixel, _, _| {
        let pixel: Laba = pixel.into_color();
        Laba::compose(vec_mask(pixel.decompose(), mask)).into_color()
    }
}

fn xform_hsva_mask<'a>(mask: &'a [Option<f32>]) -> impl 'a + FnMut(LinSrgba, u32, u32) -> LinSrgba {
    move |pixel, _, _| {
        // Convert to HSVa
        let pixel: Srgb = pixel.into_color();
        let pixel: Hsva = pixel.into_color();

        // Map pixel
        let pixel = Hsva::compose(vec_mask(pixel.decompose(), mask));

        // Undo the conversion
        let pixel: Srgb = pixel.into_color();
        pixel.into_color()
    }
}

fn vec_mask<'a, E, I>(components: I, mask: &'a [Option<E>]) -> impl Iterator<Item = E> + 'a
where
    E: Copy + Num,
    I: 'a + IntoIterator<Item = E>,
{
    components
        .into_iter()
        .zip(mask.iter())
        .map(|(comp, mask)| if let Some(mask) = *mask { mask } else { comp })
}
