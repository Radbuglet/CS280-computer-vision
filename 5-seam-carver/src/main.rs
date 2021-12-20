#![allow(dead_code)]

pub mod carver;
pub mod util;

fn main() {
    use crate::carver::{carve_vertical, sobel, LowestDerivative};
    use crate::util::{
        luma_to_rgba, vec4_to_rgba, CollectArrayError, FmtDisplayIter, IterCollectArrayExt,
        IterTryCollectExt, Kernel, KernelRect, Timer, VecKernel, VecRemoveExt,
    };
    use cgmath::{Vector2, Vector4, VectorSpace};
    use clap::{App, Arg};
    use image::{open, Rgba, RgbaImage};
    use std::path::Path;

    // TODO: If someone ever returns to this driver, *please* rewrite it with a task system.

    // === Strings === //
    const ARG_IMG_PATH_HINT: &str = "path";

    fn fmt_carve_msg(over_what: &str) -> String {
        format!(
            "Emits the seams used by the carver over {}. The color of the seams determine when they \
             were carved, with green being earlier than red.",
            over_what
        )
    }

    // === Parsing utilities === //
    #[derive(Debug, Copy, Clone)]
    struct DimComp {
        is_rel: bool,
        val: i32,
    }

    fn parse_dim(arg: &str) -> Result<(DimComp, DimComp), String> {
        const FORM_ERR: &str =
            "Argument must take the form `WIDTHxHEIGHT`. See help for more details.";

        // Split up components
        let [left, right] = arg
            .split("x")
            .try_collect_array()
            .map_err(|_| FORM_ERR.to_string())?;

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

    fn parse_debug_view_targets(arg: &str) -> Result<(&Path, Vec<u32>), String> {
        const FORM_ERR: &str =
            "Argument must take the form `path/to/image.png` or `path/to/image.png:1,2,3`. \
             See help for more details.";

        let (path, emit_at) = match arg.split(":").try_collect_array() {
            Ok([path, right]) => {
                let emit_at = right
                    .split(",")
                    .map(|part| u32::from_str_radix(part, 10))
                    .try_collect()
                    .map_err(|_| FORM_ERR.to_string())?;

                (path, emit_at)
            }
            Err(CollectArrayError::TooSmall(0)) => (arg, vec![0]),
            Err(_) => return Err(FORM_ERR.to_string()),
        };

        let path = Path::new(path);
        if path.file_name().is_none() || path.extension().is_none() {
            return Err(FORM_ERR.to_string());
        }

        Ok((path, emit_at))
    }

    // === App definition === //
    let args = App::new("Seam Carver")
        .author(clap::crate_authors!())
        .version(clap::crate_version!())
        .arg(
            Arg::with_name("timings")
                .short("v")
                .long("timings")
                .help("Displays the timings of the operations."),
        )
        // Simple use arguments
        .arg(
            Arg::with_name("input")
                .short("i")
                .long("in")
                .value_name(ARG_IMG_PATH_HINT)
                .help("Path to the image to be resized.")
                .required(true),
        )
        .arg(
            Arg::with_name("to_size")
                .short("s")
                .long("size")
                .value_name("WIDTHxHEIGHT")
                .help("The dimensions to which the image will be resized.")
                .long_help(
                    "The dimensions to which the image will be resized. \
                     Components are absolute by default but can be made \
                     relative with a leading `?` (e.g. `?20x?-30`) and \
                     preserving with `P` (e.g. `300xP`).",
                )
                .validator(|arg| {
                    parse_dim(arg.as_str())?;
                    Ok(())
                })
                .required(true),
        )
        .arg(
            Arg::with_name("output")
                .short("o")
                .long("out")
                .value_name(ARG_IMG_PATH_HINT)
                .help("Output image path. Omitting this argument will disable output saving."),
        )
        // Debug emit flags
        .arg(
            Arg::with_name("emit_sobel")
                .short("W")
                .long("emit-sobel")
                .value_name(ARG_IMG_PATH_HINT)
                .help("Emits the result of the sobel filter, which determines the 'utility' of each \
                       pixel, at specified carving steps. Place a colon followed by a list of numbers \
                       (e.g. \"--emit-seams=foo.png:1,2,3\") to specify when in the resize these seam \
                       images should be emitted.")
                .validator(|arg| {
                    parse_debug_view_targets(arg.as_str())?;
                    Ok(())
                }),
        )
        .arg(
            Arg::with_name("emit_seams_original")
                .long("emit-seams-on-original")
                .value_name(ARG_IMG_PATH_HINT)
                .help(fmt_carve_msg("the original image").as_str()),
        )
        .arg(
            Arg::with_name("emit_seams_weights")
                .short("S")
                .long("emit-seams")
                .value_name(ARG_IMG_PATH_HINT)
                .help(fmt_carve_msg("an image of the weights").as_str()),
        )
        .get_matches();

    // === Command handling === //
    if args.is_present("timings") {
        Timer::enable_printing();
    }

    // Collect arguments
    let p_input_path = args.value_of("input").unwrap();
    let (to_size_x, to_size_y) = parse_dim(args.value_of("to_size").unwrap()).unwrap();
    let p_output_path = args.value_of("output");
    let mut p_emit_sobel = args
        .value_of("emit_sobel")
        .map(|arg| parse_debug_view_targets(arg).unwrap());
    let p_emit_seams_original = args.value_of("emit_seams_original");
    let p_emit_seams_weights = args.value_of("emit_seams_weights");

    // Load image
    let mut image = open(p_input_path).unwrap().into_rgba8();
    let from_size = image.size();

    // Validate size parameters
    let to_size = Vector2::new(
        if to_size_x.is_rel { from_size.x } else { 0 } + to_size_x.val,
        if to_size_y.is_rel { from_size.y } else { 0 } + to_size_y.val,
    );

    if to_size.y != from_size.y {
        eprintln!(
            "Warning: Conversion heights must match up for the time being. \
             (wants resize from {} to {})",
            from_size.y, to_size.y
        );
    }

    if to_size.x > from_size.x {
        eprintln!(
            "Error: Target width must be less than source width for the time being. \
             (wants resize from {} to {})",
            from_size.x, to_size.x
        );
        return;
    }

    if to_size.x <= 0 {
        eprintln!(
            "Error: Target width must be greater than 0. \
             (wants resize from {} to {})",
            from_size.x, to_size.x
        );
        return;
    }

    let i_max = from_size.x - to_size.x;

    // Validate sobel debug parameters
    let starting_sobel = luma_to_rgba(&sobel(&image));

    if let Some((base_path, emit_at)) = &mut p_emit_sobel {
        // Sort for efficiency later on.
        emit_at.sort_by(|a, b| a.cmp(b).reverse());

        // Remove duplicates
        emit_at.keep_where(|left, elem| left.last().copied() != Some(*elem));

        // Handle 0 as a special case.
        if emit_at.last().copied() == Some(0) {
            starting_sobel.save(base_path).unwrap();
            emit_at.pop();
        }

        // Validate indices
        let bad_indices = emit_at
            .iter()
            .rev()
            .copied()
            .take_while(|emit_at| *emit_at > i_max as u32);

        if bad_indices.clone().next().is_some() {
            eprintln!(
                "Error: Specified invalid `--emit-sobel` emission indices: {} (there are only {} step{})",
                FmtDisplayIter { iter: bad_indices, sep: ", " },
                i_max,
                if i_max == 1 { "" } else { "s" }
            );
            return;
        }
    }

    // Setup seams tracking if necessary
    struct SeamState<'a> {
        out: RgbaImage,
        map: VecKernel<usize>,
        path: &'a str,
    }

    impl<'a> SeamState<'a> {
        fn new(image: &RgbaImage, out: RgbaImage, path: &'a str) -> Self {
            Self {
                out,
                map: VecKernel::from_fn(image.size(), |pos| image.encode_pos(pos)),
                path,
            }
        }

        fn update(&mut self, seam: &LowestDerivative, i: i32, i_max: i32) {
            // Update the seam-space to original-space map
            self.map = carve_vertical(&self.map, seam.iter());

            // Paint the seam in the debug view
            let out_size = self.out.size();
            let mut seam_iter = seam.iter();
            let color = vec4_to_rgba(
                Vector4::new(0., 1., 0., 1.)
                    .lerp(Vector4::new(1., 0., 0., 1.), i as f32 / i_max as f32),
            );
            for y in (0..out_size.y).rev() {
                let x = seam_iter.next().unwrap();
                let seam_pos = Vector2::new(x, y);
                let world_pos = *self.map.get(seam_pos);
                let world_pos = self.out.decode_pos(world_pos);
                self.out.put(world_pos, color);
            }
        }

        fn save(&self) {
            self.out.save(self.path).unwrap();
        }
    }

    let mut state_seams_original =
        p_emit_seams_original.map(|path| SeamState::new(&image, image.clone(), path));

    let mut state_seams_weights =
        p_emit_seams_weights.map(|path| SeamState::new(&image, starting_sobel, path));

    // Main pass
    {
        let _outer = Timer::start("main");
        for i in 0..i_max {
            let _inner = Timer::start("resize_pass");

            // Run a sobel filter across the image. We cannot reuse the same sobel filter
            // across iterations and update it with the same seam because doing so would
            // inaccurately reflect the modified neighbors.
            let sobel = sobel(&image);

            // Clone the sobel image if we're expected to save it on this pass.
            let sobel_save_clone = if let Some((base_path, passes)) = &mut p_emit_sobel {
                // Check if this is a desired pass.
                if passes.last().map(|val| *val as i32) == Some(i) {
                    passes.pop();

                    // If it is, clone the image and pass along the base path.
                    Some((base_path, sobel.clone()))
                } else {
                    None
                }
            } else {
                None
            };

            // Calculate the lowest weighted seam in the image
            let seam = LowestDerivative::find(sobel);

            // Save sobel filter if requested
            if let Some((base_path, sobel)) = sobel_save_clone {
                // Update the image with the chosen seam
                let mut sobel = luma_to_rgba(&sobel);
                let mut seam_iter = seam.iter();
                for y in (0..sobel.size().y).rev() {
                    let x = seam_iter.next().unwrap();
                    sobel.put(Vector2::new(x, y), Rgba([255, 0, 0, 255]));
                }

                // Create a new file name
                let new_path = base_path.with_file_name(format!(
                    "{}-{}.{}",
                    base_path.file_stem().unwrap().to_string_lossy(),
                    i,
                    base_path.extension().unwrap().to_string_lossy(),
                ));

                // Save the image
                sobel.save(new_path).unwrap();
            }

            // Update the seams debug images if requested
            {
                let _timer = Timer::start("update_seams");

                if let Some(state) = &mut state_seams_original {
                    state.update(&seam, i, i_max);
                }

                if let Some(state) = &mut state_seams_weights {
                    state.update(&seam, i, i_max);
                }
            }

            // Carve out the seam from the main image
            image = carve_vertical(&image, seam.iter());
        }
    }

    // Save artifacts
    if let Some(output_path) = p_output_path {
        image.save(output_path).unwrap();
    }

    if let Some(state) = state_seams_original {
        state.save();
    }

    if let Some(state) = state_seams_weights {
        state.save();
    }

    // Print timing stats if requested
    if Timer::is_printing() {
        println!();
        Timer::print_summary();
    }
}
