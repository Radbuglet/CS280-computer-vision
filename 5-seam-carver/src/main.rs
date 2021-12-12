#![allow(dead_code)]

pub mod carver;
pub mod util;

use crate::carver::{carve_vertical, sobel, LowestDerivative};
use crate::util::{luma_to_rgba, Timer};
use image::open;

fn main() {
    Timer::enable_printing();
    let mut image = open("images/cat.png").unwrap().into_rgba8();

    let _timer = Timer::start("main");
    for _ in 0..300 {
        let _timer_inner = Timer::start("main (loop)");
        let sobel = sobel(&image);
        let seam = LowestDerivative::find(sobel.clone());
        image = carve_vertical(&image, seam.iter());
    }
    drop(_timer);

    image.save("images/cat.out.png").unwrap();
    Timer::summary();
}
