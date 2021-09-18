//! Over-designed pixel conversion utilities that should really just be a part of the image libraries
//! directly.

use image::Rgba;
use num_traits::{Num, NumCast};
use palette::rgb::RgbStandard;
use palette::white_point::WhitePoint;
use palette::{Hsva, Laba, LinSrgba, RgbHue};
use std::ops::Range;

const COMPOSE_DIM_4_ERR: &'static str = "expected an iterator with four components";

pub trait DecomposablePixel {
    type Comp;

    fn compose<I: IntoIterator<Item = Self::Comp>>(iter: I) -> Self;
    fn decompose(&self) -> [Self::Comp; 4];
}

impl<T: 'static + image::Primitive> DecomposablePixel for Rgba<T> {
    type Comp = T;

    fn compose<I: IntoIterator<Item = Self::Comp>>(iter: I) -> Self {
        let mut comps = iter.into_iter();

        let pixel = Rgba::from([
            comps.next().expect(COMPOSE_DIM_4_ERR),
            comps.next().expect(COMPOSE_DIM_4_ERR),
            comps.next().expect(COMPOSE_DIM_4_ERR),
            comps.next().expect(COMPOSE_DIM_4_ERR),
        ]);

        debug_assert!(comps.next().is_none(), "{}", COMPOSE_DIM_4_ERR);
        pixel
    }

    fn decompose(&self) -> [Self::Comp; 4] {
        [self[0], self[1], self[2], self[3]]
    }
}

impl<T: palette::Component> DecomposablePixel for LinSrgba<T> {
    type Comp = T;

    fn compose<I: IntoIterator<Item = Self::Comp>>(iter: I) -> Self {
        let mut comps = iter.into_iter();

        let pixel = Self::new(
            comps.next().expect(COMPOSE_DIM_4_ERR),
            comps.next().expect(COMPOSE_DIM_4_ERR),
            comps.next().expect(COMPOSE_DIM_4_ERR),
            comps.next().expect(COMPOSE_DIM_4_ERR),
        );

        debug_assert!(comps.next().is_none(), "{}", COMPOSE_DIM_4_ERR);
        pixel
    }

    fn decompose(&self) -> [Self::Comp; 4] {
        [self.red, self.green, self.blue, self.alpha]
    }
}

impl<W: WhitePoint, T: palette::FloatComponent> DecomposablePixel for Laba<W, T> {
    type Comp = T;

    //noinspection DuplicatedCode
    fn compose<I: IntoIterator<Item = Self::Comp>>(iter: I) -> Self {
        let mut comps = iter.into_iter();

        let pixel = Self::from_components((
            comps.next().expect(COMPOSE_DIM_4_ERR),
            comps.next().expect(COMPOSE_DIM_4_ERR),
            comps.next().expect(COMPOSE_DIM_4_ERR),
            comps.next().expect(COMPOSE_DIM_4_ERR),
        ));

        debug_assert!(comps.next().is_none(), "{}", COMPOSE_DIM_4_ERR);
        pixel
    }

    fn decompose(&self) -> [Self::Comp; 4] {
        [self.l, self.a, self.b, self.alpha]
    }
}

impl<S: RgbStandard, T: palette::FloatComponent + Into<RgbHue>> DecomposablePixel for Hsva<S, T> {
    type Comp = T;

    //noinspection DuplicatedCode
    fn compose<I: IntoIterator<Item = Self::Comp>>(iter: I) -> Self {
        let mut comps = iter.into_iter();

        let pixel = Self::from_components((
            RgbHue::from_degrees(comps.next().expect(COMPOSE_DIM_4_ERR)),
            comps.next().expect(COMPOSE_DIM_4_ERR),
            comps.next().expect(COMPOSE_DIM_4_ERR),
            comps.next().expect(COMPOSE_DIM_4_ERR),
        ));

        debug_assert!(comps.next().is_none(), "{}", COMPOSE_DIM_4_ERR);
        pixel
    }

    fn decompose(&self) -> [Self::Comp; 4] {
        [
            self.hue.to_raw_degrees(),
            self.saturation,
            self.value,
            self.alpha,
        ]
    }
}

pub fn lin_remap<A: Copy + Num + NumCast, B: Copy + Num + NumCast>(
    val: A,
    from: Range<A>,
    to: Range<B>,
) -> B {
    let from_range = (from.end - from.start).to_f64().unwrap();
    let to_range = (to.end - to.start).to_f64().unwrap();

    let val_percent = (val - from.start).to_f64().unwrap() / from_range;
    to.start + B::from(to_range * val_percent).unwrap()
}

pub fn px_img_to_pal(px: Rgba<u8>) -> LinSrgba {
    LinSrgba::compose(
        px.decompose()
            .iter()
            .copied()
            .map(|comp| lin_remap(comp, 0..u8::MAX, 0.0..1.0)),
    )
}

pub fn px_pal_to_img(px: LinSrgba) -> Rgba<u8> {
    Rgba::compose(
        px.decompose()
            .iter()
            .copied()
            .map(|comp| lin_remap(comp, 0.0..1.0, 0..u8::MAX)),
    )
}
