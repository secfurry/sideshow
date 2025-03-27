# SideShow

eInk display Badge based on the Pinorami InkyFrame.

## How does it work?

SideShow uses the built-in SD Card reader combined with the PCF RTC to run as
a super low power badge by displaying images on the eInk display.

## States

### Automatic

With the default configration, the device will "wake-up" every 15 minutes automatically
and update the display. When not processing anything, the device will attempt to
turn itself off to save power when using the JST battery connector on the InkyFrame
board.

The display update process follows the following routes:

- Increase the "current display" count (held in PCF memory) by `1`.
  - If the "current display" count is `>= 127`, reset it to `0`.
- Open the "/backgrounds" (changable by configuration) directory on the SD Card.
  - Get the current file count in this directory
  - Choose a random image from this directory and write it's parsed contents to
    the eInk display.
- Open the "/badges" (changable by configuration) directory on the SD Card,
  - Iterate through the files in the directory until one of the conditions are met.
    - File count equals the "current display" count.
    - File is the last file in the directory.
  - If the file was the last entry, the "current display" count will be set to `127`.
  - Write the selected image's parsed contents to the eInk display.
- Update the display

The dual background and "badge" images work on the concept that TGA images allow
for transparency, which is supported by our TGA parser. Transparent pixels will
__not__ be drawn and can allow for the previously drawn pixels to show through.

This selection process allows for randomized backgrounds for every "badge" displayed.

NOTE: SideShow will only read TGA-type image files. You can use `imagmagick` to
convert them easily using `convert src.jpg dst.tga`

It's also recommended that the images are the size of the eInk display (640x400 for
InkyFrame4, 600x448 for InkyFrame5) as SideShow will draw them at (0, 0) directly.

### Buttons

The button configuration can be changed but supports the following button actions:

- __Lock__: Prevent the current "badge" from being changed automatically. This will
  change the background but not the "badge". When this action is used, the top
  LEDs (Action and Network) will display the new Lock state. It's a toggle button
  to switch the Lock state.
  - Action On, Network Off: Lock is Disabled.
  - Action Off, Network On: Lock is Enabled.
- __Random__: Select a random "badge" and background and display it. This will
  override the Lock value and disable it. The "current display" count will be set
  to the current "badge" position.
- __Previous__: Select the previous "badge" and display it. The "current display"
  count will be set to the current "badge" position (`-1`). This button __does not__
  override the Lock, if set. If the current "badge" is the first entry (`0`), the
  "current display" value will be set to `127`, which wraps around the selection.
- __Next__: Select the next "badge" and display it. The "current display"
  count will be set to the current "badge" position (`+1`). This button __does not__
  override the Lock, if set. If the current "badge" is the last entry, the
  "current display" value will be set to `0`, which wraps around the selection.
- __Custom__: TODO.

The default button configuration is:

- A: Nothing
- B: Lock
- C: Random
- D: Previous
- E: Next

When a button is pressed, it's LED will light up indicating the pressed selection.

### Errors

If SideShow encounters an error, it will switch to an error state and will require it
to be power-cycled to clear it.

Error states are indicated by the Activity and Network LEDs flashing back-and-forth
every second with one of the button LEDs enabled.

The button LED indicates the type of error that occurred.

- A: __ByteFail__: Operations on the PCF stored byte failed.
- B: __LoadFail__: Image processing operation failed. This is usually due to a badly
     formatted, corrupted file or non-TGA image.
- C: __WakeFail__: Operations on the PCF wake alarm and interrupts failed.
- D: __InvalidPins__: Setup for the eInk display was not correct. Usually this error
     is due to a configuration/code error.
- E: __InvalidRoot__: Operations on the SD Card (non-image related) failed. This is
     usually due to an error with the SD Card or it's formatting. Sometimes it
     will be a fluke issue, but may require re-formatting the SD Card if the error
     occurs multiple times in a row.

## Configuration

The configuration is fairly simple. Any changes made are in `sideshow.rs` in the
following code block:

```rust
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
```

Each name is self-explanatory and any changes require a re-compilation and upload to
the device to take affect.

## Setup

To start using SideShow, you'll need to compile this repository first.

The cargo configuration should contain all the values needed to compile in the
`thumbv6m-none-eabi` target format (for the RP2040 processor). You'll need to make
sure you have the `flip-link` linker installed before compiling. To do this, use
the command `cargo install flip-link` to install it.

If a Pico debug probe is avaliable (or you can setup one,
[see here](https://mcuoneclipse.com/2022/09/17/picoprobe-using-the-raspberry-pi-pico-as-debug-probe/)),
installing `probe-rs` using `cargo install probe-rs-tools` will allow for direct
flashing of the firmware when the probe is connected to the SideShow device when
`cargo run` is called.

If you don't have a debug probe avaliable, you can convert the compiled ELF into
a U2F file using `elf2uf2-rs` (`cargo install elf2uf2-rs`) and can be loaded using
the Bootloader mode of the Pico (hold down the "Boot" button when plugging the
device in to expose the flash storage) by copying the generated file over to flash.
_Make sure to comment out the first two lines of `.cargo/config.toml` to avoid any_
_compilation errors._

### Case

Included in the `case` directory are STL files that can be used to 3d-print a
nice case to contain the device with:

- USB-C connector port
- Simple switch (for on/off)
- LiPo Battery Pack
- AdaFruit PowerBoost 1000

## InkyFrame Devices

The current code configuration is support for the InkyFrame 4. With a couple of
code and configuration changes, it can work on different devices or larger screen
sizes.

If using an InkyFrame5, build with the "inky5" feature. This will also use the
"static_large" feature for the larger screen.

See the [InkyFrame](https://github.com/secfurry/inky-frame) repository for compatibility.
