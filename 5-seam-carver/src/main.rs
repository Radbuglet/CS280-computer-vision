#![allow(dead_code)]

use cgmath::{InnerSpace, Vector2, Vector4, Zero};
use clap::{App, Arg};
use image::{open, ImageBuffer, Luma, Pixel, Rgba, RgbaImage};
use std::cmp::Ordering;
use std::ops::{Deref, DerefMut};
use std::time::Instant;

type WeightImage = ImageBuffer<Luma<f32>, Vec<f32>>;
type ImageBufferVec<P> = ImageBuffer<P, Vec<<P as Pixel>::Subpixel>>;

fn main() {
    // let version = format!(
    //     "{}.{}.{}",
    //     env!("CARGO_PKG_VERSION_MAJOR"),
    //     env!("CARGO_PKG_VERSION_MINOR"),
    //     env!("CARGO_PKG_VERSION_PATCH")
    // );
    //
    // let matches = App::new("Scene Carver")
    //     .version(version.as_str())
    //     .about(r#"Uses scene carving to resize an image. This is called "content aware scale" in apps like Photoshop."#)
    //     .arg(Arg::with_name("input")
    //         .short("in")
    //         .long("input")
    //         .value_name("FILE")
    //         .takes_value(true))
    //     .arg(Arg::with_name("size")
    //         .short("sz")
    //         .long("size")
    //         .value_name("X Y")
    //         .takes_value(true))
    //     .get_matches();

    // let mut image = open("images/cat.png").unwrap().into_rgba8();
    //
    // for _ in 0..300 {
    //     let sobel = sobel(&image);
    //     let seam = LowestDerivative::find(&sobel);
    //     // println!("{}: {:?}", seam.weight(), seam.iter().collect::<Vec<_>>());
    //     image = carve_vertical(&image, seam.iter());
    // }
    //
    // image.save("images/cat.out.png").unwrap();

    let mut image = open("images/cat.png").unwrap().into_rgba8();

    let start = Instant::now();
    for _ in 0..300 {
        let sobel = sobel(&image);
        let seam = LowestDerivative::find(&sobel);
        image = carve_vertical(&image, seam.iter());
    }

    println!("Time taken w/o IO: {:?}", start.elapsed());
    image.save("images/cat.out.png").unwrap();
}

/// Runs a simple horizontal sobel filter on the image.
/// FIXME: This should probably be a radial sobel filter in the future.
fn sobel(target: &RgbaImage) -> WeightImage {
    fn rgba_to_vec4(pixel: &Rgba<u8>) -> Vector4<f32> {
        Vector4::from(pixel.0).cast::<f32>().unwrap() / u8::MAX as f32
    }

    target.map(|pos, _| {
        let left = target.try_get_pixel_v(pos + Vector2::new(-1, 0));
        let right = target.try_get_pixel_v(pos + Vector2::new(1, 0));
        let luma = match (left, right) {
            (Some(left), Some(right)) => {
                let left = rgba_to_vec4(left);
                let right = rgba_to_vec4(right);

                (right - left).magnitude()
            }
            // (None, Some(right)) => {
            //     let center = rgba_to_vec4(center);
            //     let right = rgba_to_vec4(right);
            //
            //     (right - center).magnitude()
            // }
            // (Some(left), None) => {
            //     let center = rgba_to_vec4(center);
            //     let left = rgba_to_vec4(left);
            //
            //     (center - left).magnitude()
            // }
            // (None, None) => unreachable!(),
            _ => f32::MAX, // FIXME
        };
        Luma([luma])
    })
}

/// Carve an image vertically across a seam.
fn carve_vertical<P, I>(target: &ImageBufferVec<P>, x_list: I) -> ImageBufferVec<P>
where
    P: StaticPixel,
    I: IntoIterator<Item = i32>,
{
    let target_sz = target.size_v();
    let mut carved = ImageBufferVec::new(target_sz.x as u32 - 1, target_sz.y as u32);
    let mut x_list = x_list.into_iter();

    for y in 0..target_sz.y {
        let remove_at = x_list.next().expect("`x_list` has the wrong size!");
        let mut write_x = 0;
        for x in 0..target_sz.x {
            // Copy the pixel if we're not attempting to remove it.
            if x != remove_at {
                *carved.get_pixel_mut_v(Vector2::new(write_x, y)) =
                    *target.get_pixel_v(Vector2::new(x, y));

                write_x += 1;
            }
        }
    }

    carved
}

#[derive(Debug, Clone)]
struct LowestDerivative {
    cache: WeightImage,
    best_x: i32,
    best_weight: f32,
}

fn cmp_second_weight<T>((_, weight_a): &(T, f32), (_, weight_b): &(T, f32)) -> Ordering {
    weight_a.partial_cmp(weight_b).unwrap()
}

impl LowestDerivative {
    pub fn find(target: &WeightImage) -> LowestDerivative {
        // Fetch and validate image dimensions
        let size = target.size_v();
        debug_assert!(
            !size.is_zero(),
            "Image dimensions must be non-zero (got {:?})",
            size,
        );

        // An image storing the cached weights of every pixel. A value of *exactly* `0.0` means that the
        // pixel weight hasn't been calculated yet.
        let mut cache = WeightImage::new(target.width(), target.height());

        // Find the weight of a given pixel, modifying the `cache`.
        fn get_weight(
            pos: Vector2<i32>,
            size: Vector2<i32>,
            cache: &mut WeightImage,
            target: &WeightImage,
        ) -> f32 {
            // Return a negligible weight if the pixel is out of bounds. We return a strictly-non-zero
            // weight to ensure that the derivative
            if !(0..size.x).contains(&pos.x) || !(0..size.y).contains(&pos.y) {
                return f32::EPSILON;
            }

            let Luma([weight]) = cache.get_pixel_v(pos).clone();

            if weight != 0.0 {
                // If the weight was already computed, return it.
                weight
            } else {
                // Otherwise, compute the pixel's weight recursively.

                // Start by taking the target's base weight.
                let Luma([own_weight]) = target.get_pixel_v(pos).clone();
                debug_assert!(own_weight >= 0., "Target weights must be positive."); // FIXME: Can this be removed?

                // ...and then join it with the neighboring pixel with the least weight.
                let weight = own_weight
                    + [
                        get_weight(pos + Vector2::new(0, 1), size, cache, target),
                        get_weight(pos + Vector2::new(-1, 1), size, cache, target),
                        get_weight(pos + Vector2::new(1, 1), size, cache, target),
                    ]
                    .iter()
                    .min_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap();

                // Update the cache and return the weight.
                *cache.get_pixel_mut_v(pos) = Luma([weight]);
                weight
            }
        }

        // Find the lowest base weight.
        let (best_x, best_weight) = (0..size.x)
            .map(|x| (x, get_weight(Vector2::new(x, 0), size, &mut cache, target)))
            .min_by(cmp_second_weight)
            .unwrap();

        Self {
            cache,
            best_x,
            best_weight,
        }
    }

    pub fn weight(&self) -> f32 {
        self.best_weight
    }

    pub fn weights(&self) -> &WeightImage {
        &self.cache
    }

    pub fn iter(&self) -> LowestDerivativeSeam<'_> {
        LowestDerivativeSeam {
            target: self,
            iter_pos: Some(Vector2::new(self.best_x, 0)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LowestDerivativeSeam<'a> {
    target: &'a LowestDerivative,
    iter_pos: Option<Vector2<i32>>,
}

impl Iterator for LowestDerivativeSeam<'_> {
    type Item = i32;

    fn next(&mut self) -> Option<Self::Item> {
        let curr_iter_pos = self.iter_pos?;

        // Find the best path in the neighboring area.
        let next_pos = [Vector2::new(-1, 1), Vector2::new(0, 1), Vector2::new(1, 1)]
            .iter()
            .filter_map(|rel| {
                let rel_pos = curr_iter_pos + rel;
                Some((rel_pos, self.target.cache.try_get_pixel_v(rel_pos)?.0[0]))
            })
            .min_by(cmp_second_weight)
            .map(|(next_pos, _)| next_pos);

        // Move there
        Some(std::mem::replace(&mut self.iter_pos, next_pos)?.x)
    }
}

/// Extension methods to make [ImageBuffer] play nicer with [cgmath].
trait ImageBufferExt {
    type Pixel;

    fn size_v(&self) -> Vector2<i32>;
    fn contains_pos(&self, pos: Vector2<i32>) -> bool;

    fn try_get_pixel_v(&self, pos: Vector2<i32>) -> Option<&Self::Pixel>;
    fn try_get_pixel_mut_v(&mut self, pos: Vector2<i32>) -> Option<&mut Self::Pixel>;
    fn get_pixel_v(&self, pos: Vector2<i32>) -> &Self::Pixel;
    fn get_pixel_mut_v(&mut self, pos: Vector2<i32>) -> &mut Self::Pixel;

    fn map<ToP, F>(&self, fn_: F) -> ImageBufferVec<ToP>
    where
        ToP: Pixel + 'static,
        ToP::Subpixel: 'static,
        F: FnMut(Vector2<i32>, &Self::Pixel) -> ToP;
}

impl<P, C> ImageBufferExt for ImageBuffer<P, C>
where
    P: StaticPixel,
    C: Deref<Target = [P::Subpixel]> + DerefMut,
{
    type Pixel = P;

    fn size_v(&self) -> Vector2<i32> {
        Vector2::new(self.width() as i32, self.height() as i32)
    }

    fn contains_pos(&self, pos: Vector2<i32>) -> bool {
        (0..(self.width() as i32)).contains(&pos.x) && (0..(self.height() as i32)).contains(&pos.y)
    }

    fn try_get_pixel_v(&self, pos: Vector2<i32>) -> Option<&Self::Pixel> {
        self.contains_pos(pos).then(|| self.get_pixel_v(pos))
    }

    fn try_get_pixel_mut_v(&mut self, pos: Vector2<i32>) -> Option<&mut Self::Pixel> {
        self.contains_pos(pos).then(|| self.get_pixel_mut_v(pos))
    }

    fn get_pixel_v(&self, pos: Vector2<i32>) -> &Self::Pixel {
        self.get_pixel(pos.x as u32, pos.y as u32)
    }

    fn get_pixel_mut_v(&mut self, pos: Vector2<i32>) -> &mut Self::Pixel {
        self.get_pixel_mut(pos.x as u32, pos.y as u32)
    }

    fn map<ToP, F>(&self, mut fn_: F) -> ImageBufferVec<ToP>
    where
        ToP: Pixel + 'static,
        ToP::Subpixel: 'static,
        F: FnMut(Vector2<i32>, &Self::Pixel) -> ToP,
    {
        ImageBuffer::from_fn(self.width(), self.height(), |x, y| {
            fn_(Vector2::new(x as i32, y as i32), self.get_pixel(x, y))
        })
    }
}

pub trait StaticPixel: Pixel + 'static
where
    Self::Subpixel: 'static,
{
}

impl<T: 'static + Pixel> StaticPixel for T where T::Subpixel: 'static {}
