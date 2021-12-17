# PWM-Fan-controller

Control a pwm fan by a DS18B20 temperature sensor. Implemented to use a Seeeduino xiao_m0 board

## Setup:

1. Connect the fan to the required power and use a logic level converter to connect the pwm pin to a1
2. connect the DS18B20 to 3.3V power of the XIAO_m0 add a 4.7K Pull-UP and connect the data pin to pin A2

Upload the program. `--release` is required. Otherwise the onewire interface is not fast enough.

## USB

if you connect the usb cable you will get the temperature readings with a `\r\n` at the end.

## Hardware: Seeeduino XIAO

This crate provides a type-safe API for working with the [Seeed Studio
Seeeduino XIAO](http://wiki.seeedstudio.com/Seeeduino-XIAO/).

## Prerequisites

- Install the cross compile toolchain `rustup target add thumbv6m-none-eabi`
- Install the [cargo-hf2 tool](https://crates.io/crates/cargo-hf2) however your
  platform requires

## Uploading the software

- Be in the project directory
- Put your device in bootloader mode by bridging the `RST` pads _twice_ in
  quick succession. The orange LED will pulse when the device is in bootloader
  mode.
- Build and upload in one step: `cargo hf2 --release`
  - Note that if you're using an older `cargo-hf2` that you'll need to specify
    the VID/PID when flashing: `cargo hf2 --vid 0x2886 --pid 0x002f --release`

Check out [the
repository](https://github.com/atsamd-rs/atsamd/tree/master/boards/xiao_m0/examples)
for examples.
