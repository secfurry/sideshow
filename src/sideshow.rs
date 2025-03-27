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

use core::convert::Into;
use core::iter::{IntoIterator, Iterator};
use core::option::Option::{None, Some};
use core::result::Result::{self, Ok};
use core::unreachable;

use inky_frame::InkyBoard;
use inky_frame::frame::heaped::Static;
use inky_frame::frame::tga::TgaParser;
use inky_frame::frame::{Inky, InkyPins, InkyRotation};
use inky_frame::fs::{BlockDevice, Mode, Volume};
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

pub enum SideError {
    ByteFail,
    LoadFail,
    WakeFail,
    InvalidPins,
    InvalidRoot,
}

pub struct SideShow<'a, const B: usize, const W: u16, const H: u16, D: BlockDevice> {
    inky:  Inky<'a, B, W, H, Static<B>>,
    root:  &'a Volume<'a, D>,
    rand:  Rand,
    board: &'a InkyBoard<'a>,
}

#[cfg(not(feature = "inky5"))]
pub type SideShowInky<'a, D> = SideShow<'a, 128_000, 640u16, 400u16, D>;

#[cfg(feature = "inky5")]
pub type SideShowInky<'a, D> = SideShow<'a, 134_400, 600u16, 448u16, D>;

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

impl<'a, D: BlockDevice> SideShowInky<'a, D> {
    #[inline(always)]
    pub fn new(b: &'a InkyBoard<'a>, root: &'a Volume<'a, D>, r: impl Into<InkyRotation>) -> Result<SideShowInky<'a, D>, SideError> {
        SideShowInky::create(b, root, InkyPins::inky_frame4(), r)
    }
}

impl<'a, const B: usize, const W: u16, const H: u16, D: BlockDevice> SideShow<'a, B, W, H, D> {
    #[inline(always)]
    pub fn create(b: &'a InkyBoard<'a>, root: &'a Volume<'a, D>, pins: InkyPins, r: impl Into<InkyRotation>) -> Result<SideShow<'a, B, W, H, D>, SideError> {
        let mut i = Inky::new(b, b.spi_bus(), pins).map_err(|_| SideError::InvalidPins)?;
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
            out!("loop enter, time: {}", self.board.pcf().now().unwrap());
            //
            l.all_off();
            // Read PFC register, fallback to 0 if it fails.
            let s = self.board.pcf().get_byte().unwrap_or(0);
            //
            out!("PFC byte_read={s}, starting switch..");
            //
            let n = self.switch(s, l, b)?;
            //
            out!("Switch result n={n}, setting PFC byte..");
            //
            l.all_off();
            self.board.pcf().set_byte(n).map_err(|_| SideError::ByteFail)?;
            self.board.sleep(2_500);
            //
            out!("Setting PFC rtc_wake time..");
            //
            let w = self.board.set_rtc_wake(SLEEP_TIME).map_err(|_| SideError::WakeFail)?;
            unsafe { self.board.power_off() };
            // Everything after this means we're on AC power.
            //
            out!("Board is NOT on battery power, running manual sleep..");
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
            if b.read_pressed() {
                break;
            }
            v = v.saturating_sub(SLEEP_STEP);
        }
        // If nothing happens, it's an RTC wake-up. Indicate it to allow wake.
        // This will only happen when connected to AC power.
        if !b.button_any() {
            b.set(Button::RTC);
        }
    }
    #[inline(always)]
    fn background(&mut self) -> Result<(), SideError> {
        self.random_set_image(DIR_BACKGROUNDS)?;
        Ok(())
    }
    fn badge(&mut self, act: Action, cur: u8) -> Result<u8, SideError> {
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
            Action::Next | Action::Wake if n >= 0x7F => 0,      // Wrap and reset.
            Action::Next | Action::Wake => n.saturating_add(1), // Advance the count.
            Action::Prev if n == 0 => 0x7F,                     // Reset to the max.
            Action::Prev => n.saturating_sub(1),                // Reduce the count.
            _ => unreachable!(),                                // Can't happen.
        };
        let i = {
            let d = self.root.dir_open(DIR_BADGES).map_err(|_| SideError::LoadFail)?;
            // Use the 'peekable' iter so we can check if the number goes out of
            // bounds so we can fix the max.
            let mut v = d
                .list()
                .map_err(|_| SideError::LoadFail)?
                .into_iter()
                .filter(|e| e.as_ref().is_ok_and(|v| v.is_file()))
                .peekable();
            let mut i = 0u8;
            // Use a loop so we can pull back to make sure we catch the end value.
            let mut f = unsafe {
                loop {
                    let e = v.next().ok_or(SideError::LoadFail)?.map_err(|_| SideError::LoadFail)?;
                    // If the next one is None, that means we're at the end.
                    if i == k || v.peek().is_none() || i >= 0x7F {
                        break e;
                    }
                    i = i.saturating_add(1);
                }
                .into_file(&self.root, Mode::READ)
                .map_err(|_| SideError::LoadFail)?
                .into_reader()
                .unwrap_unchecked()
                // SAFETY: If opened with 'Mode::READ', 'into_reader' never
                // fails.
            };
            self.inky
                .set_with(|x| x.set_image(0, 0, TgaParser::new(&mut f)?))
                .map_err(|_| SideError::LoadFail)?;
            // If 'i' is less than 'k', that means we hit the limit of the reads
            // so we should set the value to the max for a reset.
            if i < k || v.peek().is_none() { 0x7F } else { i }
        };
        Ok((cur & 0x80) | i)
    }
    fn random_set_image(&mut self, dir: &str) -> Result<usize, SideError> {
        let d = self.root.dir_open(dir).map_err(|_| SideError::LoadFail)?;
        let mut l = d.list().map_err(|_| SideError::LoadFail)?;
        let n = l.into_iter_mut().filter(|e| e.as_ref().is_ok_and(|v| v.is_file())).count();
        l.reset(&d).map_err(|_| SideError::LoadFail)?;
        let i = self.rand.rand_u32n(n as u32) as usize;
        let e = l
            .into_iter_mut()
            .filter(|e| e.as_ref().is_ok_and(|v| v.is_file()))
            .nth(i)
            .map(|v| v.ok())
            .flatten();
        let mut f = unsafe {
            match e {
                Some(v) => v,
                None => return Ok(i),
            }
            .into_file(&self.root, Mode::READ)
            .map_err(|_| SideError::LoadFail)?
            .into_reader()
            .unwrap_unchecked()
            // SAFETY: If opened with 'Mode::READ', 'into_reader' never fails.
        };
        self.inky
            .set_with(|x| x.set_image(0, 0, TgaParser::new(&mut f)?))
            .map_err(|_| SideError::LoadFail)?;
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
                    sel & 0x7F
                } else {
                    // Indicate lock is on.
                    l.network.on();
                    l.activity.off();
                    sel | 0x80
                };
                // Let the user know it was changed.
                self.board.sleep(2_000);
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
        let r = self.badge(a, sel)?;
        l.activity.off();
        self.inky.update();
        Ok(r)
    }
}

#[inline]
pub fn sideshow_error(e: SideError) -> ! {
    let i = InkyBoard::get();
    let l = i.leds();
    l.all_off();
    match e {
        SideError::ByteFail => l.a.on(),
        SideError::LoadFail => l.b.on(),
        SideError::WakeFail => l.c.on(),
        SideError::InvalidPins => l.d.on(),
        SideError::InvalidRoot => l.e.on(),
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

#[inline(always)]
pub fn sideshow(r: impl Into<InkyRotation>) -> ! {
    let b = InkyBoard::get();
    let d = b.sd_card();
    let v = d.root().unwrap_or_else(|_| sideshow_error(SideError::InvalidRoot));
    // Signal an issue if we crash after here.
    b.leds().a.on();
    b.leds().e.on();
    SideShowInky::new(&b, &v, r)
        .and_then(|mut x| x.run())
        .unwrap_or_else(sideshow_error)
}

#[cfg(feature = "debug")]
mod debug {
    extern crate core;
    extern crate rpsp;

    use core::cell::UnsafeCell;
    use core::fmt::{Arguments, Write};
    use core::marker::Sync;
    use core::option::Option::{self, None};

    use rpsp::uart::Uart;

    static DEBUG: DebugPort = DebugPort::empty();

    struct DebugPort(UnsafeCell<Option<Uart>>);

    impl DebugPort {
        #[inline(always)]
        const fn empty() -> DebugPort {
            DebugPort(UnsafeCell::new(None))
        }

        #[inline(always)]
        fn port(&self) -> &mut Uart {
            unsafe { &mut *self.0.get() }.get_or_insert_with(|| rpsp::uart_debug())
        }
    }

    unsafe impl Sync for DebugPort {}

    #[inline(always)]
    pub(super) fn output(args: Arguments<'_>) {
        let _ = DEBUG.port().write_fmt(args);
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
