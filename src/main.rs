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

#![no_std]
#![no_main]
#![no_implicit_prelude]

extern crate core;
extern crate rpsp;

mod sideshow;

#[rpsp::entry]
fn main() -> ! {
    // Rotation Values
    // - 0: Rotate0   (Buttons on Top)
    // - 1: Rotate90  (Buttons on Left)
    // - 2: Rotate180 (Buttons on Bottom, Default)
    // - 3: Rotate270 (Buttons on Right)
    sideshow::sideshow(0x2u8)
}

#[panic_handler]
fn panic(_p: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}
