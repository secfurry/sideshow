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
- Open the `/backgrounds` directory _(changable by configuration)_ on the SD Card.
  - Get the current file count in this directory
  - Choose a random image from this directory and write it's parsed contents to
    the eInk display buffer.
- Open the `/badges` directory _(changable by configuration)_ on the SD Card,
  - Iterate through the files in the directory until one of the following conditions are met.
    - File count equals the "current display" count.
    - File is the last file in the directory.
  - If the file was the last entry, the "current display" count will be set to `127`.
  - Write the selected image's parsed contents to the eInk display buffer.
- Update the eInk display.

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

- __Lock__: Prevent the current "Badge" from being changed automatically. This will
  change the background but not the "Badge". When this action is used, the top
  LEDs (Action and Network) will display the new Lock state. It's a toggle button
  to switch the Lock state.
  - Action __On__, Network __Off__: Lock is Disabled.
  - Action __Off__, Network __On__: Lock is Enabled.
- __Random__: Select a random "Badge" and background and display it. This will
  override the Lock value and disable it. The "current display" count will be set
  to the current "Badge" position.
- __Previous__: Select the previous "Badge" and display it. The "current display"
  count will be set to the current "Badge" position (`-1`). This button __does not__
  override the Lock, if set. If the current "Badge" is the first entry (`0`), the
  "current display" value will be set to `127`, which wraps around the selection.
- __Next__: Select the next "Badge" and display it. The "current display"
  count will be set to the current "Badge" position (`+1`). This button __does not__
  override the Lock, if set. If the current "Badge" is the last entry, the
  "current display" value will be set to `0`, which wraps around the selection.
- __Custom__: TODO.

The default button configuration is:

- __A__: Nothing
- __B__: Lock
- __C__: Random
- __D__: Previous
- __E__: Next

When a button is pressed, it's LED will light up indicating the pressed selection.

### Errors

If SideShow encounters an error, it will switch to an error state and will require it
to be power-cycled to clear it.

Error states are indicated by the Activity and Network LEDs flashing back-and-forth
every second. Which button LEDs are lit up indicate the type of error that occurred.

The work by [@ticky](https://github.com/ticky) has allowed for expansion of the
error types for better debugging. The resulting error code will be in 5-bit binary.
The table mapping of the LEDs to the errors is listed below.

#### Error Descriptions

- __Byte__: Operations on the PCF stored byte failed.
- __Wake__: Operations on the PCF wake alarm and interrupts failed.
- __InvalidPins__: Setup for the eInk display was not correct. Usually this error
     is due to a configuration/code error.
- __InvalidRoot__: Operations on the SD Card (non-image related) failed. This is
     usually due to an error with the SD Card or it's formatting. Sometimes it
     will be a fluke issue, but may require re-formatting the SD Card if the error
     occurs multiple times in a row.
- __Badge/DirOpen__: Generic error occurred when trying to open the Badge directory.
- __Badge/DirNotFound__: The Badge directory could not be found.
- __Badge/DirNotADir__: The Badge directory was found, but it's type was not a directory.
- __Badge/DirList__: Reading the Badge directory listing failed.
- __Badge/DirListReset__: Resetting and re-reading the Badge directory listing failed.
- __Badge/DirIter__: Walking through the Badge directory listing failed.
- __Badge/FileOpen__: Opening the selected Badge file failed. (Before Parsing).
- __Badge/ImageIo__: Reading the selected Badge file failed. (During Parsing, but
     not format related).
- __Badge/ImageType__: The selected Badge image type was not a valid TGA file.
- __Badge/ImageRead__: Generic parsing/reading error occurred when reading the selected
     Badge file.
- __Badge/ImageParse__: The selected Badge image could not be parsed due to improperly
     returned TGA data. (Corrupted or badly formatted file?).
- __Background/DirOpen__: Generic error occurred when trying to open the Background
     directory.
- __Background/DirNotFound__: The Background directory could not be found.
- __Background/DirNotADir__: The Background directory was found, but it's type was
     not a directory.
- __Background/DirList__: Reading the Background directory listing failed.
- __Background/DirListReset__: Resetting and re-reading the Background directory
     listing failed.
- __Background/DirIter__: Walking through the Background directory listing failed.
- __Background/FileOpen__: Opening the selected Background file failed. (Before Parsing).
- __Background/ImageIo__: Reading the selected Background file failed. (During Parsing,
     but not format related).
- __Background/ImageType__: The selected Background image type was not a valid TGA file.
- __Background/ImageRead__: Generic parsing/reading error occurred when reading the selected
     Background file.
- __Background/ImageParse__: The selected Background image could not be parsed due to
     improperly returned TGA data. (Corrupted or badly formatted file?).

#### Error Mapping Table

To display the error codes, the button LEDs will light up to display a 5-bit binary
number. _(A = 16, B = 8, C = 4, D = 2, E = 1)_.

| Error                   | Number Value | LED Indicator |
| ----------------------- | ------------ | ------------- |
| Byte                    |            0 |     [None]    |
| Wake                    |            1 |           E   |
| InvalidPins             |            2 |         D     |
| InvalidRoot             |            3 |         D E   |
| Badge/DirOpen           |            4 |       C       |
| Badge/DirNotFound       |            5 |       C   E   |
| Badge/DirNotADir        |            6 |       C D     |
| Badge/DirList           |            7 |       C D E   |
| Badge/DirListReset      |            8 |     B         |
| Badge/DirIter           |            9 |     B     E   |
| Badge/FileOpen          |           10 |     B   D     |
| Badge/ImageIo           |           11 |     B   D E   |
| Badge/ImageType         |           12 |     B C       |
| Badge/ImageRead         |           13 |     B C   E   |
| Badge/ImageParse        |           14 |     B C D     |
| Background/DirOpen      |           16 |   A           |
| Background/DirNotFound  |           17 |   A       E   |
| Background/DirNotADir   |           18 |   A     D     |
| Background/DirList      |           19 |   A     D E   |
| Background/DirListReset |           20 |   A   C       |
| Background/DirIter      |           21 |   A   C   E   |
| Background/FileOpen     |           22 |   A   C D     |
| Background/ImageIo      |           23 |   A   C D E   |
| Background/ImageType    |           24 |   A B         |
| Background/ImageRead    |           25 |   A B     E   |
| Background/ImageParse   |           26 |   A B   D     |

_Due to numbering, most Background related errors will have the "A" LED enabled._

#### Critical Errors

There are two odd "critical" type errors that will only happen during initialization.
The code for them is [here](src/sideshow.rs#L490). __This error type does not use
__the Activity or Network LEDs.__

The two conditions are:

- __A | B__: Stack overflow in critical section. This error is _super rare_. If
   you receive this error, attempt to run Sideshow with the `static` or `static_large`
   feature flags. If that still fails, please leave an [Issue](https://github.com/secfurry/inky-frame/issues/new?title=Stack+Overflow+Critical).
- __D | E__: SDCard initilization error. This error occurs when the SDCard does
   not respond properly to the `INIT (CMD0)` command and may indicate a bad SDCard
   or one that does not support SPI. If you receive this error, try another SDCard
   if avaliable. If you can't or the other one fails with the same error, please
   leave an [Issue](https://github.com/secfurry/inky-frame/issues/new?title=SDCard+Init+Critical)
   with information on the SDCard and it's manufactor, size and class markings
   (eg: C with the number, U with the number, etc.).

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

## Bugs

- Files named from MacOS are not correctly found by their name, even when correct.
  _Thanks to [@ticky](https://github.com/ticky) for the finding__

### SDCard Bugs

_Copied from the [inky-frame](https://github.com/secfurry/inky-frame) README._

Some SDCards don't support SPI mode or don't initialize properly. I'm not
100% sure if it's a protocol issue or something else. These cards return
`READY (0)` when asked to go into `IDLE (1)` mode. They'll work fine on PCs.

These SDCards work fine on some of my Ender 3 3D Printers, _which use Arduino's_
_SDCard library_ and have the same initializing sequence. But other devices, like
the Flipper Zero, don't work with them either.

You'll know if it fails as it won't get past the initialization phase and basically
"freezes" and does not respond with the "D" and "E" LEDs on. __This error type__
__does not use the Activity or Network LEDs.__

If you have a SDCard that has issues also, please leave me an [Issue](https://github.com/secfurry/inky-frame/issues/new?title=SDCard+Init+Critical)
with information on the SDCard and it's manufactor, size and class markings
(eg: C with the number, U with the number, etc.) so I can test further.

SDCards verified to __not__ work:

- [These SDHC Class 10/U1 Cards](https://www.amazon.com/dp/B07XJVFVSJ)
