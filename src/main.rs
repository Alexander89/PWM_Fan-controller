//! Uses an external interrupt to blink an LED.
//!
//! You need to connect a button between D12 and ground. Each time the button
//! is pressed, the LED will count the total number of button presses so far.
#![no_std]
#![no_main]

use core::cell::RefCell;
use cortex_m::{interrupt::Mutex, peripheral::NVIC};
use cortex_m_rt::entry;
use gpio::{Pin, PullUpInterrupt};
use xiao_m0::{
    hal::{
        clock::GenericClockController,
        eic::{
            pin::{ExtInt9, Sense},
            EIC,
        },
        gpio::v2::{self as gpio},
        pac::{self, interrupt, CorePeripherals, Peripherals},
        prelude::*,
    },
    Led1, Led2,
};

type RefMutOpt<T> = Mutex<RefCell<Option<T>>>;
static LED_1: RefMutOpt<Led1> = Mutex::new(RefCell::new(None));
static LED_2: RefMutOpt<Led2> = Mutex::new(RefCell::new(None));

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

    let pins = xiao_m0::Pins::new(peripherals.PORT);
    cortex_m::interrupt::free(|cs| {
        let mut l: Led1 = pins.led1.into();
        l.set_high().unwrap();
        LED_1.borrow(cs).replace(Some(l));
        let mut l: Led2 = pins.led2.into();
        l.set_high().unwrap();
        LED_2.borrow(cs).replace(Some(l));
    });

    let generator = clocks.gclk0();
    let eic_clock = clocks.eic(&generator).unwrap();
    let mut eic = EIC::init(&mut peripherals.PM, eic_clock, peripherals.EIC);

    let button: Pin<_, PullUpInterrupt> = pins.a9.into();
    let mut extint = ExtInt9::new(button);
    extint.sense(&mut eic, Sense::FALL);
    extint.enable_interrupt(&mut eic);
    extint.filter(&mut eic, true);

    // Enable EIC interrupt in the NVIC
    unsafe {
        core.NVIC.set_priority(interrupt::EIC, 1);
        NVIC::unmask(interrupt::EIC);
    }

    loop {}
}

#[interrupt]
fn EIC() {
    cortex_m::interrupt::free(|cs| {
        let mut l = LED_1.borrow(cs).borrow_mut();
        if l.is_some() {
            l.as_mut().unwrap().toggle().unwrap();
        }
        // Increase the counter and clear the interrupt.
        unsafe {
            // Accessing registers from interrupts context is safe
            let eic = &*pac::EIC::ptr();
            eic.intflag.modify(|_, w| w.extint9().set_bit());
        }
    });
}

#[cfg(not(test))]
#[inline(never)]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    use cortex_m::asm::nop;

    loop {
        for _ in 0..0xfffff {
            nop();
        }
        cortex_m::interrupt::free(|cs| {
            let mut l = LED_2.borrow(cs).borrow_mut();
            if l.is_some() {
                let _res = l.as_mut().unwrap().set_low();
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
        });
    }
}
