use crate::util::{Kernel, KernelRect, Timer, WeightImage};
use cgmath::{InnerSpace, Vector2, Vector4, Zero};
use image::{Luma, Rgba, RgbaImage};
use std::cmp::Ordering;

/// Runs a simple horizontal sobel filter on the image.
pub fn sobel(target: &RgbaImage) -> WeightImage {
    fn rgba_to_vec4(pixel: &Rgba<u8>) -> Vector4<f32> {
        Vector4::from(pixel.0).cast::<f32>().unwrap() / u8::MAX as f32
    }

    let _timer = Timer::start("sobel");
    target.map(|pos, _| {
        let left = target.try_get(pos + Vector2::new(-1, 0));
        let right = target.try_get(pos + Vector2::new(1, 0));
        let luma = match (left, right) {
            (Some(left), Some(right)) => {
                let left = rgba_to_vec4(left);
                let right = rgba_to_vec4(right);

                (right - left).magnitude()
            }
            _ => f32::MAX,
        };
        Luma([luma])
    })
}

/// Carve an image vertically across a seam.
pub fn carve_vertical<K, I>(target: &K, x_list: I) -> K
where
    K: Kernel,
    I: IntoIterator<Item = i32>,
{
    let _timer = Timer::start("carve_vertical");
    let target_sz = target.size();
    let mut carved = K::new(target_sz - Vector2::new(1, 0));
    let mut x_list = x_list.into_iter();

    for y in (0..target_sz.y).rev() {
        let remove_at = x_list.next().expect("`x_list` has the wrong size!");
        let mut write_x = 0;
        for x in 0..target_sz.x {
            // Copy the pixel if we're not attempting to remove it.
            if x != remove_at {
                carved.put(Vector2::new(write_x, y), *target.get(Vector2::new(x, y)));
                write_x += 1;
            }
        }
    }

    carved
}

#[derive(Debug, Clone)]
pub struct LowestDerivative {
    target: WeightImage,
    best_x: i32,
    best_weight: f32,
}

fn cmp_second_weight<T>((_, weight_a): &(T, f32), (_, weight_b): &(T, f32)) -> Ordering {
    weight_a.partial_cmp(weight_b).unwrap()
}

impl LowestDerivative {
    pub fn find(mut target: WeightImage) -> LowestDerivative {
        let _timer = Timer::start("LowestDerivative::find");

        // Fetch and validate image dimensions
        let size = target.size();
        debug_assert!(
            !size.is_zero(),
            "Image dimensions must be non-zero (got {:?})",
            size,
        );

        // Cascade minimum seam weights
        for y in 0..size.y {
            for x in 0..size.x {
                let pos = Vector2::new(x, y);
                let weight = target.get(pos).0[0]
                    + [
                        target.try_get(pos + Vector2::new(-1, -1)),
                        target.try_get(pos + Vector2::new(0, -1)),
                        target.try_get(pos + Vector2::new(1, -1)),
                    ]
                    .into_iter()
                    .filter_map(|luma| {
                        if let Some(Luma([luma])) = luma {
                            Some(*luma)
                        } else {
                            None
                        }
                    })
                    .min_by(|a, b| a.partial_cmp(&b).unwrap())
                    .unwrap_or(0.);

                *target.get_mut(pos) = Luma([weight]);
            }
        }

        // Find the lowest base weight.
        let (best_x, best_weight) = (0..size.x)
            .map(|x| (x, target.get_pixel(x as u32, (size.y - 1) as u32).0[0]))
            .min_by(cmp_second_weight)
            .unwrap();

        Self {
            target,
            best_x,
            best_weight,
        }
    }

    pub fn weight(&self) -> f32 {
        self.best_weight
    }

    pub fn weights(&self) -> &WeightImage {
        &self.target
    }

    pub fn iter(&self) -> LowestDerivativeSeam<'_> {
        LowestDerivativeSeam {
            target: self,
            iter_pos: Some(Vector2::new(self.best_x, (self.target.height() - 1) as i32)),
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
        let next_pos = [
            Vector2::new(-1, -1),
            Vector2::new(0, -1),
            Vector2::new(1, -1),
        ]
        .iter()
        .filter_map(|rel| {
            let rel_pos = curr_iter_pos + rel;
            Some((rel_pos, self.target.target.try_get(rel_pos)?.0[0]))
        })
        .min_by(cmp_second_weight)
        .map(|(next_pos, _)| next_pos);

        // Move there
        Some(std::mem::replace(&mut self.iter_pos, next_pos)?.x)
    }
}
