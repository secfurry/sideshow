// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
//

#![no_implicit_prelude]

extern crate core;

extern crate inky_frame;
extern crate rpsp;

use core::convert::{From, Into};
use core::hint::unlikely;
use core::iter::{IntoIterator, Iterator};
use core::option::Option::{None, Some};
use core::panic::PanicInfo;
use core::result::Result::{self, Err, Ok};
use core::unreachable;

use inky_frame::InkyBoard;
use inky_frame::frame::heaped::Static;
use inky_frame::frame::tga::{ImageError, TgaParser};
use inky_frame::frame::{Inky, InkyPins, InkyRotation};
use inky_frame::fs::{BlockDevice, DeviceError, Mode, Volume};
use inky_frame::hw::{Button, Buttons, Leds};
use rpsp::MayFail;
use rpsp::rand::Rand;

use crate::out;

// =================== [   Configuration   ] ===================
/// Time in milliseconds) to wait for a button check. Only
/// takes affect when NOT on battery power.
const SLEEP_STEP: u32 = 50u32;
/// Time (in seconds) to wake up and change the current
/// badge and/or background.
const SLEEP_TIME: u32 = 15u32 * 60u32;

/// Directory name in the SD Card root to get the badge
/// images from.
const DIR_BADGES: &str = "/badges";
/// Directory name in the SD Card root to get the background
/// images from.
const DIR_BACKGROUNDS: &str = "/backgrounds";

/// Action to return when the 'A' button is pressed.
const BUTTON_A: Action = Action::None;
/// Action to return when the 'B' button is pressed.
const BUTTON_B: Action = Action::Lock;
/// Action to return when the 'C' button is pressed.
const BUTTON_C: Action = Action::Rand;
/// Action to return when the 'D' button is pressed.
const BUTTON_D: Action = Action::Prev;
/// Action to return when the 'E' button is pressed.
const BUTTON_E: Action = Action::Next;
// =================== [ Configuration End ] ===================

#[repr(u8)]
pub enum SideError {
    Byte                   = 0x00u8,
    Wake                   = 0x01u8,
    InvalidPins            = 0x02u8,
    InvalidRoot            = 0x03u8,
    // NOTE(sf): If we encapsulated the LoadError inside this error (SideError),
    //           the size of the enum would be 2b instead of 1b, which only needs
    //           1b to be expressed.
    //
    //           This is more ugly, but it saves some space. *shrug*.
    //
    // Badge Errors
    BadgeDirOpen           = 0x04u8,
    BadgeDirNotFound       = 0x05u8,
    BadgeDirNotADir        = 0x06u8,
    BadgeDirList           = 0x07u8,
    BadgeDirListReset      = 0x08u8,
    BadgeDirIter           = 0x09u8,
    BadgeImageIo           = 0x0Bu8,
    BadgeImageType         = 0x0Cu8,
    BadgeImageRead         = 0x0Du8,
    BadgeImageParse        = 0x0Eu8,
    // Background Errors
    BackgroundDirOpen      = 0x10u8,
    BackgroundDirNotFound  = 0x11u8,
    BackgroundDirNotADir   = 0x12u8,
    BackgroundDirList      = 0x13u8,
    BackgroundDirListReset = 0x14u8,
    BackgroundDirIter      = 0x15u8,
    BackgroundImageIo      = 0x17u8,
    BackgroundImageType    = 0x18u8,
    BackgroundImageRead    = 0x19u8,
    BackgroundImageParse   = 0x1Au8,
    // Generic Panic
    Panic                  = 0x1Fu8,
}

pub struct SideShow<'a, const B: usize, const W: u16, const H: u16, D: BlockDevice> {
    inky:  Inky<'a, B, W, H, Static<B>>,
    root:  &'a Volume<'a, D>,
    rand:  Rand,
    board: &'a InkyBoard<'a>,
}

#[cfg(feature = "inky5")]
pub type SideShowInky<'a, D> = SideShow<'a, 134_400, 600u16, 448u16, D>;
#[cfg(not(feature = "inky5"))]
pub type SideShowInky<'a, D> = SideShow<'a, 128_000, 640u16, 400u16, D>;

enum Action {
    None,
    Next,
    Prev,
    Rand,
    Wake,
    Lock,
    // TODO(sf): Custom Actions
    // Custom,
}
enum LoadError {
    DirOpen,
    DirNotFound,
    DirNotADir,
    DirList,
    DirListReset,
    DirIter,
    ImageIo,
    ImageType,
    ImageRead,
    ImageParse,
}

impl SideError {
    #[inline]
    fn badge(e: LoadError) -> SideError {
        match e {
            LoadError::DirIter => SideError::BadgeDirIter,
            LoadError::DirList => SideError::BadgeDirList,
            LoadError::DirOpen => SideError::BadgeDirOpen,
            LoadError::ImageIo => SideError::BadgeImageIo,
            LoadError::ImageRead => SideError::BadgeImageRead,
            LoadError::ImageType => SideError::BadgeImageType,
            LoadError::DirNotADir => SideError::BadgeDirNotADir,
            LoadError::ImageParse => SideError::BadgeImageParse,
            LoadError::DirNotFound => SideError::BadgeDirNotFound,
            LoadError::DirListReset => SideError::BadgeDirListReset,
        }
    }
    #[inline]
    fn background(e: LoadError) -> SideError {
        match e {
            LoadError::DirIter => SideError::BackgroundDirIter,
            LoadError::DirList => SideError::BackgroundDirList,
            LoadError::DirOpen => SideError::BackgroundDirOpen,
            LoadError::ImageIo => SideError::BackgroundImageIo,
            LoadError::ImageRead => SideError::BackgroundImageRead,
            LoadError::ImageType => SideError::BackgroundImageType,
            LoadError::DirNotADir => SideError::BackgroundDirNotADir,
            LoadError::ImageParse => SideError::BackgroundImageParse,
            LoadError::DirNotFound => SideError::BackgroundDirNotFound,
            LoadError::DirListReset => SideError::BackgroundDirListReset,
        }
    }
}
impl<'a, D: BlockDevice> SideShowInky<'a, D> {
    #[inline]
    pub fn new(b: &'a InkyBoard<'a>, root: &'a Volume<'a, D>, r: impl Into<InkyRotation>) -> Result<SideShowInky<'a, D>, SideError> {
        SideShowInky::create(b, root, InkyPins::inky_frame4(), r)
    }
}
impl<'a, const B: usize, const W: u16, const H: u16, D: BlockDevice> SideShow<'a, B, W, H, D> {
    #[inline]
    pub fn create(b: &'a InkyBoard<'a>, root: &'a Volume<'a, D>, pins: InkyPins, r: impl Into<InkyRotation>) -> Result<SideShow<'a, B, W, H, D>, SideError> {
        let mut i = Inky::new(b, b.spi_bus(), pins).or(Err(SideError::InvalidPins))?;
        i.set_rotation(r.into());
        Ok(SideShow {
            root,
            inky: i,
            rand: Rand::new(),
            board: b,
        })
    }

    pub fn run(&mut self) -> MayFail<SideError> {
        let (l, b) = (self.board.leds(), self.board.buttons());
        loop {
            //
            out!(
                "run(): Enter, current time: {}",
                self.board.pcf().now().unwrap()
            );
            //
            l.all_off();
            // Read PFC register, fallback to 0 if it fails.
            let s = self.board.pcf().get_byte().unwrap_or(0);
            //
            out!("run(): PCF get_byte s={s}, starting switch..");
            //
            let n = self.switch(s, l, b)?;
            //
            out!("run(): Switch return n={n}, setting PCF byte..");
            //
            l.all_off();
            self.board.pcf().set_byte(n).or(Err(SideError::Byte))?;
            self.board.sleep(1_000);
            //
            out!("run(): Setting PCF rtc_wake time to {SLEEP_TIME}s..");
            //
            let w = self.board.set_rtc_wake(SLEEP_TIME).or(Err(SideError::Wake))?;
            unsafe { self.board.power_off() };
            // Everything after this means we're on AC power.
            //
            out!("run(): Board is NOT on battery power, running manual sleep..");
            //
            self.sleep(b, w);
            // Disable alarm and reset RTC state.
            let _ = self.board.pcf().alarm_clear_state();
            let _ = self.board.pcf().alarm_disable();
        }
    }

    fn sleep(&mut self, b: &mut Buttons, w: u32) {
        let mut v = w;
        while v > 0 {
            self.board.sleep(SLEEP_STEP);
            // Watch for button presses.
            if b.read().any_pressed() {
                break;
            }
            v = v.saturating_sub(SLEEP_STEP);
        }
        // If nothing happens, it's an RTC wake-up. Indicate it to allow wake.
        // This will only happen when connected to AC power.
        //
        // Basically simulates the RTC waking us up.
        if !b.any_pressed() {
            b.set_pressed(Button::RTC);
        }
    }
    #[inline]
    fn background(&mut self) -> Result<(), SideError> {
        let _ = self.random_set_image(DIR_BACKGROUNDS).map_err(SideError::background)?;
        Ok(())
    }
    fn badge(&mut self, act: Action, cur: u8) -> Result<u8, LoadError> {
        match &act {
            Action::None => return Ok(cur), // Just in case.
            // Random: Override the Badge lock and set a random one. Set this
            //         new badge position as the index, without the lock on.
            Action::Rand => return Ok(self.random_set_image(DIR_BADGES)? as u8),
            // Wake: Don't change the badge selected if the lock is on, if it's
            //       off, act like Next.
            // Next: Don't change the badge selected if the lock is on, if it's
            //       off, select the next badge, wrapping if over the badge count.
            // Prev: Don't change the badge selected if the lock is on, if it's
            //       off, select the last badge, resetting to the max if zero.
            _ => (),
        }
        let n = cur & 0x7F;
        let k = match act {
            _ if cur & 0x80 != 0 => n,                          // All stay the same when the lock is enabled.
            Action::Next | Action::Wake if n == 0x7F => 0,      // Wrap and reset.
            Action::Next | Action::Wake => n.saturating_add(1), // Advance the count.
            Action::Prev if n == 0 => 0x7F,                     // Reset to the max.
            Action::Prev => n.saturating_sub(1),                // Reduce the count.
            _ => unreachable!(),                                // Can't happen.
        };
        let i = {
            let d = self.root.dir_open(DIR_BADGES)?;
            // Use the 'peekable' iter so we can check if the number goes out of
            // bounds so we can fix the max.
            let mut v = d
                .list()
                .or(Err(LoadError::DirList))?
                .into_iter()
                .filter(|e| e.as_ref().is_ok_and(|v| !v.is_hidden() && v.is_file()))
                .peekable();
            let mut i = 0u8;
            // Use a loop so we can pull back to make sure we catch the end value.
            let mut f = unsafe {
                loop {
                    let e = v.next().ok_or(LoadError::DirIter)?.or(Err(LoadError::DirIter))?;
                    // If the next one is None, that means we're at the end.
                    if i == k || v.peek().is_none() || i >= 0x7F {
                        out!("badge(): Select image i={i}, k={k}, name=[{}]", e.name());
                        break e;
                    }
                    i = i.saturating_add(1);
                }
                .into_file(&self.root, Mode::READ)
                .unwrap_unchecked()
                .into_reader()
                .unwrap_unchecked()
                // SAFETY: If opened with 'Mode::READ', 'into_reader' and
                //         'into_file' never fails.
            };
            self.inky.set_with(|x| x.set_image(0, 0, TgaParser::new(&mut f)?))?;
            // If 'i' is less than 'k', that means we hit the limit of the reads
            // so we should set the value to the max for a reset.
            if i < k || v.peek().is_none() { 0x7F } else { i }
        };
        Ok((cur & 0x80) | i)
    }
    fn random_set_image(&mut self, dir: &str) -> Result<usize, LoadError> {
        let d = self.root.dir_open(dir)?;
        let mut l = d.list().or(Err(LoadError::DirList))?;
        let n = l
            .into_iter_mut()
            .filter(|e| e.as_ref().is_ok_and(|v| !v.is_hidden() && v.is_file()))
            .count();
        out!("random_set_image(): Count n={n} entries");
        if unlikely(l.reset(&d).is_err()) {
            return Err(LoadError::DirListReset);
        }
        let i = self.rand.rand_u32n(n as u32) as usize;
        out!("random_set_image(): Selected i={i} of n={n}");
        let e = l
            .into_iter_mut()
            .filter(|e| e.as_ref().is_ok_and(|v| !v.is_hidden() && v.is_file()))
            .nth(i)
            .map(|v| v.ok())
            .flatten();
        let mut f = unsafe {
            match e {
                Some(v) => {
                    out!("random_set_image(): Select image name=[{}]", v.name());
                    v
                },
                None => return Ok(i),
            }
            .into_file(&self.root, Mode::READ)
            .unwrap_unchecked()
            .into_reader()
            .unwrap_unchecked()
            // SAFETY: If opened with 'Mode::READ', 'into_reader' and
            //         'into_file' never fails.
        };
        self.inky.set_with(|x| x.set_image(0, 0, TgaParser::new(&mut f)?))?;
        Ok(i)
    }
    #[inline]
    fn switch(&mut self, sel: u8, l: &Leds, b: &mut Buttons) -> Result<u8, SideError> {
        // Check if any button was pressed.
        let a = match b.pressed() {
            Button::ButtonA => {
                l.a.on();
                BUTTON_A
            },
            Button::ButtonB => {
                l.b.on();
                BUTTON_B
            },
            Button::ButtonC => {
                l.c.on();
                BUTTON_C
            },
            Button::ButtonD => {
                l.d.on();
                BUTTON_D
            },
            Button::ButtonE => {
                l.e.on();
                BUTTON_E
            },
            Button::None => Action::None,
            Button::RTC | Button::External => Action::Wake,
        };
        // Signal online.
        l.activity.on();
        match a {
            Action::None => return Ok(sel),
            Action::Lock => {
                let v = if sel & 0x80 != 0 {
                    // Indicate lock is off.
                    l.network.off();
                    l.activity.on();
                    for i in 0..9 {
                        self.board.sleep(300);
                        l.activity.set_on(i % 2 == 1);
                    }
                    sel & 0x7F
                } else {
                    // Indicate lock is on.
                    l.network.on();
                    l.activity.off();
                    for i in 0..9 {
                        self.board.sleep(300);
                        l.network.set_on(i % 2 == 1);
                    }
                    sel | 0x80
                };
                // Let the user know it was changed.
                self.board.sleep(1_000);
                return Ok(v);
            },
            Action::Rand if sel == 0 => {
                l.all_on();
                self.board.sleep(2_000);
                return Ok(sel);
            },
            _ => (),
        }
        self.background()?;
        l.network.on();
        let r = self.badge(a, sel).map_err(SideError::badge)?;
        l.activity.off();
        self.inky.update();
        Ok(r)
    }
}

impl From<ImageError> for LoadError {
    #[inline]
    fn from(v: ImageError) -> LoadError {
        out!("LoadError::from(): Converting ImageError ({v:?}) to LoadError");
        match v {
            ImageError::Io(_) => LoadError::ImageIo,
            ImageError::InvalidImage => LoadError::ImageParse,
            ImageError::Empty | ImageError::InvalidColorMap => LoadError::ImageRead,
            ImageError::InvalidType(_) | ImageError::NotTGA => LoadError::ImageType,
        }
    }
}
impl From<DeviceError> for LoadError {
    #[inline]
    fn from(v: DeviceError) -> LoadError {
        out!("LoadError::from(): Converting DeviceError ({v:?}) to LoadError");
        match v {
            DeviceError::NotFound => LoadError::DirNotFound,
            DeviceError::NotADirectory => LoadError::DirNotADir,
            _ => LoadError::DirOpen,
        }
    }
}

#[inline]
pub fn sideshow_error(e: SideError) -> ! {
    let i = InkyBoard::get();
    let l = i.leds();
    l.all_off();
    // Convert the error into a number that can be displayed with
    // the LEDs.
    let v = e as u8;
    //
    // | Error                   | Number Value | LED Indicator |
    // | ----------------------- | ------------ | ------------- |
    // | Byte                    |            0 |     [None]    |
    // | Wake                    |            1 |           E   |
    // | InvalidPins             |            2 |         D     |
    // | InvalidRoot             |            3 |         D E   |
    // | Badge/DirOpen           |            4 |       C       |
    // | Badge/DirNotFound       |            5 |       C   E   |
    // | Badge/DirNotADir        |            6 |       C D     |
    // | Badge/DirList           |            7 |       C D E   |
    // | Badge/DirListReset      |            8 |     B         |
    // | Badge/DirIter           |            9 |     B     E   |
    // | Badge/FileOpen          |           10 |     B   D     | (Deprecated)
    // | Badge/ImageIo           |           11 |     B   D E   |
    // | Badge/ImageType         |           12 |     B C       |
    // | Badge/ImageRead         |           13 |     B C   E   |
    // | Badge/ImageParse        |           14 |     B C D     |
    // | Background/DirOpen      |           16 |   A           |
    // | Background/DirNotFound  |           17 |   A       E   |
    // | Background/DirNotADir   |           18 |   A     D     |
    // | Background/DirList      |           19 |   A     D E   |
    // | Background/DirListReset |           20 |   A   C       |
    // | Background/DirIter      |           21 |   A   C   E   |
    // | Background/FileOpen     |           22 |   A   C D     | (Deprecated)
    // | Background/ImageIo      |           23 |   A   C D E   |
    // | Background/ImageType    |           24 |   A B         |
    // | Background/ImageRead    |           25 |   A B     E   |
    // | Background/ImageParse   |           26 |   A B   D     |
    // | Panic                   |           32 |   A B C D E   |
    //
    out!("sideshow_error(): Error (errono {v}) received, stopping!");
    if v & 0x1 == 0x1 {
        l.e.on();
    }
    if v & 0x2 == 0x2 {
        l.d.on();
    }
    if v & 0x4 == 0x4 {
        l.c.on();
    }
    if v & 0x8 == 0x8 {
        l.b.on();
    }
    if v & 0x10 == 0x10 {
        l.a.on();
    }
    loop {
        i.sleep(1_500);
        l.network.on();
        l.activity.off();
        i.sleep(1_500);
        l.network.off();
        l.activity.on();
    }
}
#[cold]
#[inline(never)]
pub fn sideshow_panic(_v: &PanicInfo<'_>) -> ! {
    out!("PANIC OCCURRED!!");
    if let Some(_m) = _v.message().as_str() {
        out!("MESSAGE: {}", _m);
    }
    if let Some(_l) = _v.location() {
        out!("LOCATION: {};{}:{}", _l.file(), _l.line(), _l.column());
    }
    sideshow_error(SideError::Panic)
}
#[inline]
pub fn sideshow(r: impl Into<InkyRotation>) -> ! {
    let b = InkyBoard::get();
    let d = b.sd_card();
    // Signal an issue if we crash due to the SDCard init failure.
    b.leds().d.on();
    b.leds().e.on();
    let v = match d.root() {
        Err(_) => sideshow_error(SideError::InvalidRoot),
        Ok(v) => v,
    };
    // Signal an issue if we crash after here.
    b.leds().a.on();
    b.leds().b.on();
    SideShowInky::new(&b, &v, r)
        .and_then(|mut x| x.run())
        .unwrap_or_else(sideshow_error)
}

#[cfg(feature = "debug")]
mod debug {
    extern crate core;
    extern crate rpsp;

    use core::fmt::{Arguments, Debug, Formatter, Result, Write};

    use rpsp::uart_debug;

    use crate::sideshow::SideError;

    impl Debug for SideError {
        #[inline]
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            match self {
                SideError::Byte => f.write_str("Byte"),
                SideError::Wake => f.write_str("Wake"),
                SideError::InvalidPins => f.write_str("InvalidPins"),
                SideError::InvalidRoot => f.write_str("InvalidRoot"),
                SideError::BadgeDirOpen => f.write_str("Badge/DirOpen"),
                SideError::BadgeDirNotFound => f.write_str("Badge/DirNotFound"),
                SideError::BadgeDirNotADir => f.write_str("Badge/DirNotADir"),
                SideError::BadgeDirList => f.write_str("Badge/DirList"),
                SideError::BadgeDirListReset => f.write_str("Badge/DirListReset"),
                SideError::BadgeDirIter => f.write_str("Badge/DirIter"),
                SideError::BadgeImageIo => f.write_str("Badge/ImageIo"),
                SideError::BadgeImageType => f.write_str("Badge/ImageType"),
                SideError::BadgeImageRead => f.write_str("Badge/ImageRead"),
                SideError::BadgeImageParse => f.write_str("Badge/ImageParse"),
                SideError::BackgroundDirOpen => f.write_str("Background/DirOpen"),
                SideError::BackgroundDirNotFound => f.write_str("Background/DirNotFound"),
                SideError::BackgroundDirNotADir => f.write_str("Background/DirNotADir"),
                SideError::BackgroundDirList => f.write_str("Background/DirList"),
                SideError::BackgroundDirListReset => f.write_str("Background/DirListReset"),
                SideError::BackgroundDirIter => f.write_str("Background/DirIter"),
                SideError::BackgroundImageIo => f.write_str("Background/ImageIo"),
                SideError::BackgroundImageType => f.write_str("Background/ImageType"),
                SideError::BackgroundImageRead => f.write_str("Background/ImageRead"),
                SideError::BackgroundImageParse => f.write_str("Background/ImageParse"),
                SideError::Panic => f.write_str("Panic"),
            }
        }
    }

    #[inline]
    pub(super) fn output(args: Arguments<'_>) {
        let v = uart_debug();
        let _ = v.write_fmt(args);
        let _ = v.write_char(b'\n' as char);
    }

    #[macro_export]
    macro_rules! out {
        ($($arg:tt)*) => {{
            debug::output(core::format_args!($($arg)*));
        }};
    }
}
#[cfg(not(feature = "debug"))]
mod debug {
    #[macro_export]
    macro_rules! out {
        ($($arg:tt)*) => {{}};
    }
}
