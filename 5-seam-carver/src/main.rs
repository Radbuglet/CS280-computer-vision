#![allow(dead_code)]

pub mod carver;
pub mod util;

use crate::carver::{carve_vertical, sobel, LowestDerivative};
use crate::util::Timer;
use image::open;

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

    Timer::enable_printing();
    let mut image = open("images/cat.png").unwrap().into_rgba8();

    let _timer = Timer::start("main");
    for _ in 0..300 {
        let _timer_inner = Timer::start("main (loop)");
        let sobel = sobel(&image);
        let seam = LowestDerivative::find(&sobel);
        image = carve_vertical(&image, seam.iter());
    }
    drop(_timer);

    image.save("images/cat.out.png").unwrap();
    Timer::summary();
}
