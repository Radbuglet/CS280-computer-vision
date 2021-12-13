#![allow(dead_code)]

use crate::carver::{carve_vertical, sobel, LowestDerivative};
use crate::util::{luma_to_rgba, ImageBufferExt, Timer};
use cgmath::Vector2;
use image::{open, RgbaImage};

pub mod carver;
pub mod util;

fn main() {
    use clap::{App, AppSettings, Arg, SubCommand};

    // === Strings === //
    const ARG_IMG_PATH_HINT: &str = "path";

    // === Parsing utilities === //
    #[derive(Debug, Copy, Clone)]
    struct DimComp {
        is_rel: bool,
        val: i32,
    }

    fn parse_dim(arg: &str) -> Result<(DimComp, DimComp), String> {
        const FORM_ERR: &str = "Argument must take the form `WIDTHxHEIGHT`.";

        // Split up components
        let mut comps = arg.split("x");
        let left = comps.next().ok_or_else(|| FORM_ERR.to_string())?;
        let right = comps.next().ok_or_else(|| FORM_ERR.to_string())?;
        if comps.next().is_some() {
            return Err(FORM_ERR.to_string());
        }

        // Validate components
        fn parse_comp(mut comp: &str) -> Result<DimComp, String> {
            // Parse prefix
            let is_rel = match comp.chars().next() {
                Some('p' | 'P') => {
                    return if comp.len() == 1 {
                        Ok(DimComp {
                            is_rel: true,
                            val: 0,
                        })
                    } else {
                        Err(FORM_ERR.to_string())
                    }
                }
                Some('?') => {
                    comp = &comp[1..];
                    true
                }
                None => return Err(FORM_ERR.to_string()),
                _ => false,
            };

            // Parse digits
            let val = i32::from_str_radix(comp, 10).map_err(|_| FORM_ERR.to_string())?;

            Ok(DimComp { is_rel, val })
        }

        let left = parse_comp(left)?;
        let right = parse_comp(right)?;

        Ok((left, right))
    }

    // === App definition === //
    let args = App::new("Seam Carver")
        .author(clap::crate_authors!())
        .version(clap::crate_version!())
        .setting(AppSettings::SubcommandRequired)
        .arg(
            Arg::with_name("timings")
                .short("v")
                .long("timings")
                .help("Displays the timings of the operations.")
        )
        .subcommand(
            SubCommand::with_name("resize")
                .arg(
                    Arg::with_name("input")
                        .short("i")
                        .long("in")
                        .value_name(ARG_IMG_PATH_HINT)
                        .help("Path to the image to be resized.")
                        .required(true),
                )
                .arg(
                    Arg::with_name("output")
                        .short("o")
                        .long("out")
                        .value_name(ARG_IMG_PATH_HINT)
                        .help("Output image path. A value of `?` will cause this to overwrite the input image.")
                        .required(true),
                )
                .arg(
                    Arg::with_name("dbg_sobel")
                        .short("w")
                        .long("sobel")
                        .value_name(ARG_IMG_PATH_HINT)
                        .help("Emits the result of the sobel filter, which determines the 'utility' of each pixel."),
                )
                .arg(
                    Arg::with_name("dbg_seams")
                        .short("c")
                        .long("seams")
                        .value_name(ARG_IMG_PATH_HINT)
                        .help("Emits the seams used by the carver, with green seams being cheap and red ones being costly."),
                )
                .arg(
                    Arg::with_name("to_size")
                        .short("s")
                        .long("to")
                        .value_name("WIDTHxHEIGHT")
                        .help("The dimensions to which the image will be resized.")
                        .long_help("The dimensions to which the image will be resized. \
                                   Components are absolute by default but can be made \
                                   relative with a leading `?` (e.g. `?20x?-30`) and \
                                   preserving with `P` (e.g. `300xP`).")
                        .validator(|arg| {
                            parse_dim(arg.as_str())?;
                            Ok(())
                        })
                        .required(true),
                ),
        )
        .get_matches();

    // === Subcommand handling === //
    if args.is_present("timings") {
        Timer::enable_printing();
    }

    match args.subcommand() {
        ("resize", Some(args)) => {
            // Collect arguments
            let input_path = args.value_of("input").unwrap();
            let output_path = args.value_of("output").unwrap();
            let output_path = if output_path == "?" {
                input_path
            } else {
                output_path
            };
            let sobel_path = args.value_of("dbg_sobel");
            let seams_path = args.value_of("dbg_seams");
            let (to_size_x, to_size_y) = parse_dim(args.value_of("to_size").unwrap()).unwrap();

            // Load image
            let mut image = open(input_path).unwrap().into_rgba8();
            let from_size = image.size_v();

            #[rustfmt::skip]
            let to_size = Vector2::new(
                if to_size_x.is_rel { from_size.x } else { 0 } + to_size_x.val,
                if to_size_y.is_rel { from_size.y } else { 0 } + to_size_y.val
            );

            // Emit sobel filter if requested
            if let Some(sobel_path) = sobel_path {
                luma_to_rgba(&sobel(&image)).save(sobel_path).unwrap();
            }

            // Setup seams tracking if necessary
            struct SeamState {
                out: RgbaImage,
                map: Vec<usize>,
            }

            // TODO: Update this
            let _seams_state = seams_path.map(|_| SeamState {
                out: image.clone(),
                map: (0..(image.width() * image.height()))
                    .map(|i| i as usize)
                    .collect(),
            });

            // Main pass
            {
                let _outer = Timer::start("main");
                for _ in to_size.x..from_size.x {
                    let _inner = Timer::start("resize_pass");
                    let sobel = sobel(&image);
                    let seams = LowestDerivative::find(sobel);

                    image = carve_vertical(&image, seams.iter());
                }
            }
            image.save(output_path).unwrap();

            if Timer::is_printing() {
                println!();
                Timer::print_summary();
            }
        }
        _ => unreachable!(),
    }
}
