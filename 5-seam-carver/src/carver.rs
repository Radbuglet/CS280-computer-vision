use crate::util::{ImageBufferExt, ImageBufferVec, StaticPixel, Timer, WeightImage};
use cgmath::{num_traits::Zero, InnerSpace, Vector2, Vector4};
use image::{Luma, Rgba, RgbaImage};
use std::cmp::Ordering;

/// Runs a simple horizontal sobel filter on the image.
/// FIXME: This should probably be a radial sobel filter in the future.
pub fn sobel(target: &RgbaImage) -> WeightImage {
    fn rgba_to_vec4(pixel: &Rgba<u8>) -> Vector4<f32> {
        Vector4::from(pixel.0).cast::<f32>().unwrap() / u8::MAX as f32
    }

    let _timer = Timer::start("sobel");
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
pub fn carve_vertical<P, I>(target: &ImageBufferVec<P>, x_list: I) -> ImageBufferVec<P>
where
    P: StaticPixel,
    I: IntoIterator<Item = i32>,
{
    let _timer = Timer::start("carve_vertical");
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
pub struct LowestDerivative {
    cache: WeightImage,
    best_x: i32,
    best_weight: f32,
}

fn cmp_second_weight<T>((_, weight_a): &(T, f32), (_, weight_b): &(T, f32)) -> Ordering {
    weight_a.partial_cmp(weight_b).unwrap()
}

impl LowestDerivative {
    pub fn find(target: &WeightImage) -> LowestDerivative {
        let _timer = Timer::start("LowestDerivative::find");

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
