use cgmath::{Vector2, Vector4};
use image::{ImageBuffer, Luma, Pixel, Rgba, RgbaImage};
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub type WeightImage = ImageBuffer<Luma<f32>, Vec<f32>>;
pub type ImageBufferVec<P> = ImageBuffer<P, Vec<<P as Pixel>::Subpixel>>;

// === ImageBuffer wrappers === //

pub trait KernelRect {
    fn size(&self) -> Vector2<i32>;

    fn dim(&self) -> usize {
        let size = self.size();
        size.x as usize * size.y as usize
    }

    fn contains_pos(&self, pos: Vector2<i32>) -> bool {
        let size = self.size();
        (0..size.x).contains(&pos.x) && (0..size.y).contains(&pos.y)
    }

    fn try_encode_pos(&self, pos: Vector2<i32>) -> Option<usize> {
        if self.contains_pos(pos) {
            Some(self.encode_pos(pos))
        } else {
            None
        }
    }

    fn encode_pos(&self, pos: Vector2<i32>) -> usize {
        debug_assert!(self.contains_pos(pos));
        let size = self.size();
        pos.x as usize + (pos.y as usize * size.x as usize)
    }

    fn decode_pos(&self, pos: usize) -> Vector2<i32> {
        debug_assert!(pos < self.dim());
        let size = self.size();
        let x = pos % size.x as usize;
        let y = pos / size.x as usize;
        Vector2::new(x as i32, y as i32)
    }
}

impl KernelRect for Vector2<i32> {
    fn size(&self) -> Vector2<i32> {
        *self
    }
}

pub trait Kernel: Sized + KernelRect + Clone {
    type Pixel: 'static + Copy;

    fn new(size: Vector2<i32>) -> Self;

    fn from_fn<F>(size: Vector2<i32>, handler: F) -> Self
    where
        F: FnMut(Vector2<i32>) -> Self::Pixel;

    fn map<K, F>(&self, mut fn_: F) -> K
    where
        K: Kernel,
        F: FnMut(Vector2<i32>, &Self::Pixel) -> K::Pixel,
    {
        K::from_fn(self.size(), |pos| fn_(pos, self.get(pos)))
    }

    fn try_get(&self, pos: Vector2<i32>) -> Option<&Self::Pixel> {
        if self.contains_pos(pos) {
            Some(self.get(pos))
        } else {
            None
        }
    }

    fn try_get_mut(&mut self, pos: Vector2<i32>) -> Option<&mut Self::Pixel> {
        if self.contains_pos(pos) {
            Some(self.get_mut(pos))
        } else {
            None
        }
    }

    fn get(&self, pos: Vector2<i32>) -> &Self::Pixel;

    fn get_mut(&mut self, pos: Vector2<i32>) -> &mut Self::Pixel;

    fn put(&mut self, pos: Vector2<i32>, value: Self::Pixel) -> Self::Pixel {
        std::mem::replace(self.get_mut(pos), value)
    }
}

impl<P: StaticPixel> Kernel for ImageBuffer<P, Vec<P::Subpixel>> {
    type Pixel = P;

    fn new(size: Vector2<i32>) -> Self {
        ImageBuffer::new(size.x as u32, size.y as u32)
    }

    fn from_fn<F>(size: Vector2<i32>, mut handler: F) -> Self
    where
        F: FnMut(Vector2<i32>) -> Self::Pixel,
    {
        ImageBuffer::from_fn(size.x as u32, size.y as u32, |x, y| {
            handler(Vector2::new(x as i32, y as i32))
        })
    }

    fn get(&self, pos: Vector2<i32>) -> &Self::Pixel {
        self.get_pixel(pos.x as u32, pos.y as u32)
    }

    fn get_mut(&mut self, pos: Vector2<i32>) -> &mut Self::Pixel {
        self.get_pixel_mut(pos.x as u32, pos.y as u32)
    }
}

impl<P: StaticPixel> KernelRect for ImageBuffer<P, Vec<P::Subpixel>> {
    fn size(&self) -> Vector2<i32> {
        Vector2::new(self.width() as i32, self.height() as i32)
    }
}

pub trait StaticPixel: Pixel + 'static
where
    Self::Subpixel: 'static,
{
}

impl<T: 'static + Pixel> StaticPixel for T where T::Subpixel: 'static {}

#[derive(Debug, Clone)]
pub struct VecKernel<P> {
    width: u32,
    pixels: Vec<P>,
}

impl<P: 'static + Default + Copy> Kernel for VecKernel<P> {
    type Pixel = P;

    fn new(size: Vector2<i32>) -> Self {
        Self {
            width: size.x as u32,
            pixels: (0..size.dim()).map(|_| Default::default()).collect(),
        }
    }

    fn from_fn<F>(size: Vector2<i32>, mut handler: F) -> Self
    where
        F: FnMut(Vector2<i32>) -> Self::Pixel,
    {
        let mut pixels = Vec::with_capacity(size.x as usize * size.y as usize);
        for i in 0..size.dim() {
            pixels.push(handler(size.decode_pos(i)));
        }

        Self {
            width: size.x as u32,
            pixels,
        }
    }

    fn get(&self, pos: Vector2<i32>) -> &Self::Pixel {
        debug_assert!(self.contains_pos(pos));
        &self.pixels[self.encode_pos(pos)]
    }

    fn get_mut(&mut self, pos: Vector2<i32>) -> &mut Self::Pixel {
        debug_assert!(self.contains_pos(pos));
        let index = self.encode_pos(pos);
        &mut self.pixels[index]
    }
}

impl<P: 'static + Default + Copy> KernelRect for VecKernel<P> {
    fn size(&self) -> Vector2<i32> {
        Vector2::new(
            self.width as i32,
            self.pixels.len() as i32 / self.width as i32,
        )
    }
}

// === Color magic === //

pub fn luma_to_rgba(target: &WeightImage) -> RgbaImage {
    let mut image = RgbaImage::new(target.width(), target.height());
    let mut weights = target
        .enumerate_pixels()
        .map(|(x, y, luma)| (x, y, luma.0[0]))
        .collect::<Vec<_>>();
    weights.sort_by(|(_, _, a), (_, _, b)| a.partial_cmp(b).unwrap());

    for (i, (x, y, _)) in weights.iter().enumerate() {
        let luma = i as f32 / weights.len() as f32;
        let luma = (luma * 256.) as u8;
        image.put_pixel(*x, *y, Rgba([luma, luma, luma, 255]));
    }

    image
}

pub fn vec4_to_rgba(vec: Vector4<f32>) -> Rgba<u8> {
    Rgba([
        (vec.x * 256.) as u8,
        (vec.y * 256.) as u8,
        (vec.z * 256.) as u8,
        (vec.w * 256.) as u8,
    ])
}

// === Debug tools === //

struct TimerGlobal {
    print: bool,
    indent: usize,
    accumulator: HashMap<String, Duration>,
}

lazy_static! {
    static ref TIMER: Mutex<TimerGlobal> = Mutex::new(TimerGlobal {
        print: false,
        indent: 0,
        accumulator: HashMap::new(),
    });
}

const TAB_SEQ: &str = "    ";

#[derive(Debug, Clone)]
pub struct Timer<'a> {
    label: &'a str,
    time: Instant,
}

impl<'a> Timer<'a> {
    pub fn start(label: &'a str) -> Self {
        let mut global = TIMER.lock().unwrap();

        if global.print {
            // Print header
            println!(
                "{}+ {}",
                FmtRepeat {
                    seq: TAB_SEQ,
                    count: global.indent,
                },
                label
            );
            global.indent += 1;
        }

        // Construct timer
        Self {
            label,
            time: Instant::now(),
        }
    }

    pub fn enable_printing() {
        TIMER.lock().unwrap().print = true;
    }

    pub fn disable_printing() {
        TIMER.lock().unwrap().print = false;
    }

    pub fn is_printing() -> bool {
        TIMER.lock().unwrap().print
    }

    pub fn print_summary() {
        let global = TIMER.lock().unwrap();
        println!("=== Timing Summary === ");
        for (name, accum) in &global.accumulator {
            println!("{}: {:?}", name, accum);
        }
        println!("====================== ");
    }
}

impl Drop for Timer<'_> {
    fn drop(&mut self) {
        let elapsed = self.time.elapsed();
        let mut global = TIMER.lock().unwrap();

        if let Some(accum) = global.accumulator.get_mut(self.label) {
            *accum += elapsed;
        } else {
            global.accumulator.insert(self.label.to_string(), elapsed);
        }

        if global.print {
            global.indent -= 1;

            println!(
                "{}  Elapsed: {:?}",
                FmtRepeat {
                    seq: TAB_SEQ,
                    count: global.indent,
                },
                elapsed,
            );
        }
    }
}

#[derive(Debug, Clone)]
pub struct FmtRepeat<S> {
    pub seq: S,
    pub count: usize,
}

impl<S: Display> Display for FmtRepeat<S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for _ in 0..self.count {
            self.seq.fmt(f)?
        }
        Ok(())
    }
}
