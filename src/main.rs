//! Uses an external interrupt to blink an LED.
//!
//! You need to connect a button between a9 and ground. Each time the button
//! is pressed, the LED will toggle.
//!
#![no_std]
#![no_main]

use core::cell::RefCell;
use cortex_m::{interrupt::Mutex, peripheral::NVIC};
use cortex_m_rt::entry;
use hal::{
    clock::ClockGenId,
    clock::GenericClockController,
    eic::pin::ExtInt5,
    eic::{
        pin::{ExtInt9, Sense},
        EIC,
    },
    gpio::v2::{Pin, PullUpInterrupt},
    pac::{self, interrupt, CorePeripherals, Peripherals},
    prelude::*,
};
use xiao_m0::{hal, Led0, Led1, Led2};

type RefMutOpt<T> = Mutex<RefCell<Option<T>>>;
static LED_1: RefMutOpt<Led1> = Mutex::new(RefCell::new(None));
static LED_2: RefMutOpt<Led2> = Mutex::new(RefCell::new(None));

#[entry]
fn main() -> ! {
    run()
}
fn run() -> ! {
    let mut peripherals = Peripherals::take().unwrap();
    let mut core = CorePeripherals::take().unwrap();
    let mut clocks = GenericClockController::with_internal_8mhz(
        peripherals.GCLK,
        &mut peripherals.PM,
        &mut peripherals.SYSCTRL,
        &mut peripherals.NVMCTRL,
    );

    let pins = xiao_m0::Pins::new(peripherals.PORT);
    let mut l0: Led0 = pins.led0.into();
    cortex_m::interrupt::free(|cs| {
        let mut l: Led1 = pins.led1.into();
        l.set_high().unwrap();
        LED_1.borrow(cs).replace(Some(l));
        let mut l: Led2 = pins.led2.into();
        l.set_high().unwrap();
        LED_2.borrow(cs).replace(Some(l));
    });

    let generator = clocks
        .configure_gclk_divider_and_source(
            ClockGenId::GCLK2,
            1,
            pac::gclk::genctrl::SRC_A::OSC8M,
            false,
        )
        .unwrap();

    let eic_clock = clocks.eic(&generator).unwrap();
    let mut eic = EIC::init(&mut peripherals.PM, eic_clock, peripherals.EIC);

    let p7: Pin<_, PullUpInterrupt> = pins.a7.into();
    let mut extint9 = ExtInt9::new(p7);

    let p9: Pin<_, PullUpInterrupt> = pins.a9.into();
    let mut extint5 = ExtInt5::new(p9);

    extint9.sense(&mut eic, Sense::FALL);
    extint9.enable_interrupt(&mut eic);
    extint9.filter(&mut eic, true);

    extint5.sense(&mut eic, Sense::FALL);
    extint5.enable_interrupt(&mut eic);
    extint5.filter(&mut eic, true);

    // Enable EIC interrupt in the NVIC
    unsafe {
        core.NVIC.set_priority(interrupt::EIC, 2);
        NVIC::unmask(interrupt::EIC);
    }

    loop {
        let _ = l0.toggle();

        for _ in 0..0xfffff {
            cortex_m::asm::nop();
        }
    }
}

// #[interrupt]
// fn EVSYS() {
//     cortex_m::interrupt::free(|cs| {
//         let mut l = LED_1.borrow(cs).borrow_mut();
//         if l.is_some() {
//             l.as_mut().unwrap().toggle().unwrap();
//         }
//     });
// }

#[interrupt]
fn EIC() {
    cortex_m::interrupt::free(|cs| {
        let mut l1 = LED_1.borrow(cs).borrow_mut();
        let mut l2 = LED_2.borrow(cs).borrow_mut();

        // Increase the counter and clear the interrupt.
        let eic = unsafe { &*pac::EIC::ptr() };
        // Accessing registers from interrupts context is safe
        if eic.intflag.read().extint5().bit_is_set() {
            if l1.is_some() {
                l1.as_mut().unwrap().toggle().unwrap();
            }
            eic.intflag.modify(|_, w| w.extint5().set_bit());
        }

        if eic.intflag.read().extint9().bit_is_set() {
            if l2.is_some() {
                l2.as_mut().unwrap().toggle().unwrap();
            }
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
