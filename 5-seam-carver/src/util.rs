use cgmath::Vector2;
use image::{ImageBuffer, Luma, Pixel};
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::ops::{Deref, DerefMut};
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub type WeightImage = ImageBuffer<Luma<f32>, Vec<f32>>;
pub type ImageBufferVec<P> = ImageBuffer<P, Vec<<P as Pixel>::Subpixel>>;

/// Extension methods to make [ImageBuffer] play nicer with [cgmath].
pub trait ImageBufferExt {
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
                    seq: '\t',
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

    pub fn summary() {
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
                    seq: '\t',
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
