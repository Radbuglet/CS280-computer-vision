#![feature(decl_macro)]

// TODO: Write-ups

use anyhow::Context;
use image::{ImageBuffer, RgbaImage, GrayImage, Rgba, Luma};
use cgmath::Vector2;
use std::cell::Cell;
use std::ops::Deref;
use num_integer::Integer;
use cgmath::num_traits::abs;

fn main() {
    if let Err(err) = task_timer!("main", main_fallible()) {
        panic!("{}", err);
    }
}

fn main_fallible() -> anyhow::Result<()> {
    println!(
        "Current working directory: {}",
        std::env::current_dir().map_or("<NO CWD FOUND>".to_string(), |path| path
            .to_string_lossy()
            .into_owned()
            .to_string())
    );

    // Load images
    let in_color_monkey = task_timer!("load image", load_image("images/in/color-monke.jpg")?);

    // Construct common blur filter
    let blur_filter = {
        const BR: f32 = 1. / 25.;
        new_filter_hardcoded(5, 5, &[
            BR, BR, BR, BR, BR,
            BR, BR, BR, BR, BR,
            BR, BR, BR, BR, BR,
            BR, BR, BR, BR, BR,
            BR, BR, BR, BR, BR,
        ])
    };

    // === Exercise 1 === //

    task_timer!("exercise 1, filter 1 - movement filter", {
        apply_filter(
            &in_color_monkey,
            &luma_to_rgba(&new_filter_movement(2, Vector2::new(2, 0))),
            &mut pixel_getter_solid,
        ).save("images/exercise_1_filter_1.png")?;
    });

    task_timer!("exercise 1, filter 2 - brighten filter", {
        apply_filter(
            &in_color_monkey,
            &new_filter_hardcoded(3, 3, &[
                0., 0., 0.,
                0., 2., 0.,
                0., 0., 0.,
            ]),
            &mut pixel_getter_solid,
        ).save("images/exercise_1_filter_2.png")?;
    });

    task_timer!("exercise 1, filter 3 - sharpen filter", {
        apply_filter(
            &in_color_monkey,
            &new_filter_hardcoded(3, 3, &[
                -0.11, -0.11, -0.11,
                -0.11,  1.88, -0.11,
                -0.11, -0.11, -0.11,
            ]),
            &mut pixel_getter_solid,
        ).save("images/exercise_1_filter_3.png")?;
    });

    // === Exercise 2 === //

    // Wow, very chromatic aberration.
    task_timer!("exercise 2 - filter independent planes", {
        apply_filter(
            &in_color_monkey,
            &new_filter_planes(
                &new_filter_movement(4, Vector2::new(0, 4)),
                &new_filter_movement(4, Vector2::new(4, 0)),
                &new_filter_movement(4, Vector2::new(0, 0)),
            ),
            &mut pixel_getter_solid,
        ).save("images/exercise_2.png")?;
    });

    // === Exercise 3 === //

    task_timer!("exercise 3, filter 1 - square blue", {
        apply_filter(
            &in_color_monkey,
            &blur_filter,
            &mut pixel_getter_solid,
        ).save("images/exercise_3_square_blur.png")?;
    });

    task_timer!("exercise 3, filter 2 - square edge blue", {
        let edge = new_filter_edge_blur(Vector2::new(11, 11));
        GrayImage::from_fn(edge.width(), edge.height(), |x, y| {
            Luma([(edge.get_pixel(x, y)[0] * 255.) as u8])
        }).save("images/exercise_3_edge_blur_filter.png")?;

        task_timer!("apply", {
            apply_filter(
                &in_color_monkey,
                &luma_to_rgba(&edge),
                &mut pixel_getter_solid,
            ).save("images/exercise_3_edge_blur.png")?;
        });
    });

    // === Advanced exercise 1 === //

    task_timer!("advanced exercise 1, move wrap", {
        apply_filter(
            &in_color_monkey,
            &luma_to_rgba(&new_filter_movement(11, Vector2::new(11, 5))),
            &mut pixel_getter_wrap,
        ).save("images/exercise_adv_1_move_wrap.png")?;
    });

    task_timer!("advanced exercise 1, blur wrap", {
        apply_filter(
            &in_color_monkey,
            &blur_filter,
            &mut pixel_getter_wrap,
        ).save("images/exercise_adv_1_blur_wrap.png")?;
    });

    task_timer!("advanced exercise 1, blur mirror", {
        apply_filter(
            &in_color_monkey,
            &blur_filter,
            &mut pixel_getter_mirror,
        ).save("images/exercise_adv_1_blur_mirror.png")?;
    });

    // === Advanced exercise 2 === //
    // Fun optimization: build and run with "--release"!

    Ok(())
}

// === Task timing === //

thread_local! { static TASK_DEPTH: Cell<u32> = Cell::new(0) }

macro task_timer($name:expr, $cb:expr) {{
    // Resolve name expression so we don't run it multiple times.
    let name = $name;

    // Construct indent prefix
    let indent = TASK_DEPTH.with(|depth| {
        let indent = (0..depth.get()).map(|_| "\t").collect::<String>();
        depth.set(depth.get() + 1);
        indent
    });

    // Run task
    println!("{}+ Starting task \"{}\"...", indent, name);
    let start = ::std::time::Instant::now();
    let ret = $cb;
    println!("{}- Finished task \"{}\" in {:?}", indent, name, start.elapsed());

    // Dedent
    TASK_DEPTH.with(|depth| depth.set(depth.get() - 1));

    ret
}}

// === Image construction === //

type LumaFilter<B = Vec<f32>> = ImageBuffer<Luma<f32>, B>;
type RgbaFilter<B = Vec<f32>> = ImageBuffer<Rgba<f32>, B>;

/// Loads an image at a path, annotating any IO errors with the path of the file.
fn load_image(path: &str) -> anyhow::Result<RgbaImage> {
    Ok(image::open(path)
        .with_context(|| format!("Could not open '{}'. Is the CWD correct?", path))?
        .into_rgba8())
}

/// Converts a Luma filter to an RGB filter, both being backed by `f32`s.
fn luma_to_rgba<B: Deref<Target = [f32]>>(gray: &LumaFilter<B>) -> RgbaFilter {
    ImageBuffer::from_fn(gray.width(), gray.height(), |x, y| {
        let brightness = gray.get_pixel(x, y);
        Rgba::from([brightness[0], brightness[0], brightness[0], 1.0])
    })
}

/// Directly creates an [RgbaFilter] from a hardcoded array of intensities.
fn new_filter_hardcoded(width: u32, height: u32, pixels: &[f32]) -> RgbaFilter {
    luma_to_rgba(&LumaFilter::from_raw(width, height, pixels).expect("Illegal image size."))
}

/// Merges several [LumaFilter] color planes into a single [RgbaFilter].
fn new_filter_planes<B: Deref<Target = [f32]>>(
    r: &LumaFilter<B>, g: &LumaFilter<B>, b: &LumaFilter<B>
) -> RgbaFilter {
    // Check size
    {
        let r_size = Vector2::new(r.width(), r.height());
        let g_size = Vector2::new(g.width(), g.height());
        let b_size = Vector2::new(b.width(), b.height());
        assert!(r_size == g_size && g_size == b_size, "All image planes must have identical sizes!");
    }

    // Merge image planes
    ImageBuffer::from_fn(r.width(), r.height(), move |x, y| {
        Rgba::from([
            r.get_pixel(x, y)[0],
            g.get_pixel(x, y)[0],
            b.get_pixel(x, y)[0],
            1.0
        ])
    })
}

/// Constructs a new [LumaFilter] where each pixel takes its source from the pixel at `self + rel`.
fn new_filter_movement(max_comp: u32, rel: Vector2<i32>) -> LumaFilter {
    let dim = max_comp * 2 + 1;

    LumaFilter::from_fn(dim, dim, move |x, y| {
        let pos = Vector2::new(x as i32, y as i32) - Vector2::new(max_comp as _, max_comp as _);
        if pos == rel {
            Luma::from([1.])
        } else {
            Luma::from([0.])
        }

    })
}

/// Constructs a weird border blur [LumaFilter].
fn new_filter_edge_blur(size: Vector2<u32>) -> LumaFilter {
    let inner_ranges = size.map(|max| 1.min(max) .. max.checked_sub(1).unwrap_or(0));
    let outer_area = size.x * size.y;
    let inner_area = (inner_ranges.x.end - inner_ranges.x.start) * (inner_ranges.y.end - inner_ranges.y.start);
    let border_area = outer_area - inner_area;
    let pixel_average = 1. / (border_area as f32);

    LumaFilter::from_fn(size.x, size.y, move |x, y| {
        if inner_ranges.x.contains(&x) && inner_ranges.y.contains(&y) {
            Luma([0.])
        } else {
            Luma([pixel_average])
        }
    })
}

// === Filter implementations === //

fn pixel_getter_solid(image: &RgbaImage, x: i32, y: i32) -> Rgba<u8> {
    if x >= 0 && x < image.width() as i32 && y >= 0 && y < image.height() as i32 {
        *image.get_pixel(x as u32, y as u32)
    } else {
        Rgba::from([0, 0, 0, 255])
    }
}

fn pixel_getter_wrap(image: &RgbaImage, x: i32, y: i32) -> Rgba<u8> {
    *image.get_pixel(
        x.mod_floor(&(image.width() as i32)) as u32,
        y.mod_floor(&(image.height() as i32)) as u32,
    )
}

fn pixel_getter_mirror(image: &RgbaImage, x: i32, y: i32) -> Rgba<u8> {
    let width = image.width() as i32 - 1;
    let height = image.height() as i32 - 1;

    *image.get_pixel(
        abs((x - width).mod_floor(&(width * 2)) - width) as _,
        abs((y - height).mod_floor(&(height * 2)) - height) as _,
    )
}

fn apply_filter<F>(main_view: &RgbaImage, filter_view: &RgbaFilter, getter: &mut F) -> RgbaImage
where
    F: FnMut(&RgbaImage, i32, i32) -> Rgba<u8>
{
    // Sanity check to ensure that filters are odd so we can properly center them around the subject
    // pixel.
    assert!(filter_view.width().is_odd() && filter_view.height().is_odd(), "Filter dimensions must be odd!");

    // Compute the offset from a pixel coordinate in the filter image to a pixel in the main view
    // centered at (0, 0).
    let filter_to_main_offset = Vector2::new(filter_view.width() as i32, filter_view.height() as i32) / -2;

    // For every pixel in the main image...
    ImageBuffer::from_fn(main_view.width(), main_view.height(), move |x, y| {
        // Construct an accumulator pixel with floating point components.
        let mut accum = Rgba::from([0., 0., 0., 0.]);

        // For every pixel in the filter...
        for filter_x in 0..filter_view.width() {
            for filter_y in 0..filter_view.height() {
                // Compute the corresponding position in the main image (may be out of bounds)
                let main_pos = Vector2::new(x as i32, y as i32) +
                    (Vector2::new(filter_x as i32, filter_y as i32) + filter_to_main_offset);

                // Get both pixels
                let main_px = getter(&main_view, main_pos.x, main_pos.y);
                let filter_px = filter_view.get_pixel(filter_x, filter_y);

                // Add every component to the accumulator.
                for i in 0..4 {
                    accum[i] += ((main_px[i] as f32) / 255.) * filter_px[i];
                }
            }
        }

        // Compute the pixel average
        Rgba::from([
            (accum[0] * 255.) as _,
            (accum[1] * 255.) as _,
            (accum[2] * 255.) as _,
            (accum[3] * 255.) as _
        ])
    })
}
