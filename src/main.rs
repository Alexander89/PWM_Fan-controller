//! Driver to control the fan with and DS18B20 temperature sensor

#![no_std]
#![no_main]

mod usb_serial;

use core::{cell::RefCell, convert::Infallible};
use cortex_m::interrupt::Mutex;
use cortex_m_rt::entry;
use hal::{
    clock::ClockGenId,
    clock::GenericClockController,
    pac::{self, CorePeripherals, Peripherals},
    prelude::nb::block,
    prelude::*,
    pwm::{Channel, Pwm1},
    sercom::v2::uart,
    sercom::v2::uart::{BaudMode, Oversampling},
};
use usb_serial::UsbSerial;
use xiao_m0::{
    hal::{self, delay::Delay, gpio::v2::E, pac::interrupt},
    Led0, Led1, Led2,
};

use onewire::{
    ds18b20::{self, DS18B20},
    DeviceSearch, OneWire,
};

use string_helper::f32_to_str;

type RefMutOpt<T> = Mutex<RefCell<Option<T>>>;
static LED_1: RefMutOpt<Led1> = Mutex::new(RefCell::new(None));
static LED_2: RefMutOpt<Led2> = Mutex::new(RefCell::new(None));
static mut SERIAL: Option<UsbSerial> = None;

#[entry]
fn main() -> ! {
    let mut peripherals = Peripherals::take().unwrap();
    let mut core = CorePeripherals::take().unwrap();
    let mut clocks = GenericClockController::with_internal_32kosc(
        peripherals.GCLK,
        &mut peripherals.PM,
        &mut peripherals.SYSCTRL,
        &mut peripherals.NVMCTRL,
    );
    let mut delay = Delay::new(core.SYST, &mut clocks);
    let pins = xiao_m0::Pins::new(peripherals.PORT);

    let mut l0: Led0 = pins.led0.into();

    // setup pwm
    let gclk2 = clocks
        .configure_gclk_divider_and_source(
            ClockGenId::GCLK2,
            1,
            pac::gclk::genctrl::SRC_A::OSC8M,
            false,
        )
        .unwrap();
    let clock = &clocks.tcc0_tcc1(&gclk2).unwrap();
    let mut pwm = Pwm1::new(clock, 25.khz(), peripherals.TCC1, &mut peripherals.PM);
    let _p3 = pins.a8.into_alternate::<E>();

    // setup temp sensor
    let mut ds18b20_pin = pins.a5.into_readable_output();
    let mut wire = OneWire::new(&mut ds18b20_pin, false);

    let mut search = DeviceSearch::new();
    let ds18b20 = loop {
        if let Some(device) = wire.search_next(&mut search, &mut delay).unwrap() {
            if let ds18b20::FAMILY_CODE = device.address[0] {
                break Some(DS18B20::new::<Infallible>(device).unwrap());
            }
        } else {
            panic!();
        }
    };

    // setup static LEDs
    let led1 = pins.led1;
    let led2 = pins.led2;
    cortex_m::interrupt::free(|cs| {
        let mut l: Led1 = led1.into();
        l.set_high().unwrap();
        LED_1.borrow(cs).replace(Some(l));
        let mut l: Led2 = led2.into();
        l.set_high().unwrap();
        LED_2.borrow(cs).replace(Some(l));
    });

    // Setup UART peripheral
    let mut uart = {
        let gclk0 = clocks.gclk0();
        let clock = &clocks.sercom4_core(&gclk0).unwrap();
        let pads = uart::Pads::default().rx(pins.a7).tx(pins.a6);
        uart::Config::new(&mut peripherals.PM, peripherals.SERCOM4, pads, clock.freq())
            .baud(115200.hz(), BaudMode::Fractional(Oversampling::Bits16))
            .stop_bits(uart::StopBits::OneBit)
            .collision_detection(false)
            .parity(uart::Parity::Odd)
            .enable()
    };

    // setup static serial
    let serial = {
        let usb_dm = pins.usb_dm;
        let usb_dp = pins.usb_dp;
        let usb = peripherals.USB;
        let pm = &mut peripherals.PM;
        let nvic = &mut core.NVIC;
        cortex_m::interrupt::free(|_| {
            // usb serial
            let serial = UsbSerial::init(&mut clocks, usb, pm, usb_dm, usb_dp, nvic);

            unsafe {
                SERIAL = Some(serial);
                SERIAL.as_mut().unwrap()
            }
        })
    };

    // config fan values
    let max_duty = pwm.get_max_duty();

    pwm.set_duty(Channel::_1, max_duty / 32);
    pwm.enable(Channel::_1);

    let min = max_duty / 12;
    let max = max_duty;

    let speed = |proc: u32| {
        if proc == 0 {
            0
        } else {
            min + ((max - min) * proc) / 100
        }
    };

    loop {
        // cycle state led
        let _ = l0.toggle();

        // get temp value (/ 16 to convert to float value)
        let temperature = if let Some(ds18b20) = ds18b20.as_ref() {
            // request sensor to measure temperature
            let resolution = ds18b20.measure_temperature(&mut wire, &mut delay).unwrap();

            // wait for completion, depends on resolution
            delay.delay_ms(resolution.time_ms());

            // read temperature
            ds18b20.read_temperature(&mut wire, &mut delay).unwrap()
        } else {
            0
        };
        let temp_f32 = temperature as f32 / 16.0;

        // debug print / output temperature
        let _ = block!(uart.write((temperature / 4) as u8));

        // debug print / output temperature
        let (l, bytes) = f32_to_str(temp_f32, 2);
        serial.serial_write_len(&(bytes[(12 - l)..12]), l);
        serial.serial_write(b"\r\n");

        // calc fan speed in % (max 100 %)
        let min = 25.0;
        let max = 40.0;
        let a = 100.0 / (max - min);

        let proc = ((temp_f32.max(min) - min) * a).min(100.0) as u32;

        // set fan speed
        pwm.set_duty(Channel::_1, speed(proc));
    }
}

// poll USB interface with interrupt
#[interrupt]
fn USB() {
    if let Some(serial) = unsafe { SERIAL.as_mut() } {
        serial.poll();
        let _ = serial.read_poll();
    }
}

// blink to show panic
#[inline(never)]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    use cortex_m::asm::nop;

    for _ in 0..10 {
        for _ in 0..0xfffff {
            nop();
        }
        cortex_m::interrupt::free(|cs| {
            let mut l = LED_2.borrow(cs).borrow_mut();
            if l.is_some() {
                let _res = l.as_mut().unwrap().set_low();
            }
            let mut l = LED_1.borrow(cs).borrow_mut();
            if l.is_some() {
                let _res = l.as_mut().unwrap().set_high();
            }
        });

        for _ in 0..0xfffff {
            nop();
        }
        cortex_m::interrupt::free(|cs| {
            let mut l = LED_2.borrow(cs).borrow_mut();
            if l.is_some() {
                let _res = l.as_mut().unwrap().set_high();
            }
            let mut l = LED_1.borrow(cs).borrow_mut();
            if l.is_some() {
                let _res = l.as_mut().unwrap().set_low();
            }
        });
    }
    pac::SCB::sys_reset();
}
