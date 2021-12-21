use core::cell::RefCell;

use cortex_m::interrupt::{self as interrupt_cs, Mutex};
use usb_device::{
    class::UsbClass,
    class_prelude::UsbBusAllocator,
    device::{UsbDevice, UsbDeviceBuilder, UsbVidPid},
};
use usbd_serial::{SerialPort, USB_CLASS_CDC};
use xiao_m0::hal::{clock::GenericClockController, target_device::NVIC, usb::UsbBus};
use xiao_m0::{pac, UsbDm, UsbDp};

use string_helper::u32_to_str;

static mut BUS_ALLOCATOR: Option<UsbBusAllocator<UsbBus>> = None;

pub struct UsbSerial {
    bus: UsbDevice<'static, UsbBus>,
    serial: Mutex<RefCell<SerialPort<'static, UsbBus>>>,
}

impl UsbSerial {
    #[allow(dead_code)]
    pub fn poll(&mut self) {
        cortex_m::interrupt::free(|cs| {
            let mut serial = self.serial.borrow(cs).borrow_mut();
            let mut classes: [&mut dyn UsbClass<UsbBus>; 1] = [&mut *serial];
            self.bus.poll(&mut classes);
        })
    }

    #[allow(dead_code)]
    pub fn read_poll(&mut self) -> core::result::Result<(usize, [u8; 100]), ()> {
        cortex_m::interrupt::free(|cs| {
            let mut buf = [0u8; 100];
            let mut serial = self.serial.borrow(cs).borrow_mut();
            match serial.read(&mut buf) {
                Ok(count) if count > 0 => Ok((count, buf)),
                _ => Err(()),
            }
        })
    }

    #[allow(dead_code)]
    pub fn serial_write(&mut self, bytes: &[u8]) {
        self.serial_write_len(&bytes, bytes.len())
    }

    #[allow(dead_code)]
    pub fn serial_write_num(&mut self, num: usize) {
        let (_len, bytes) = u32_to_str(num as u32);
        self.serial_write_len(&(bytes as [u8; 12]), 12)
    }

    #[allow(dead_code)]
    pub fn serial_write_len(&mut self, bytes: &[u8], len: usize) {
        cortex_m::interrupt::free(|cs| {
            let mut serial = self.serial.borrow(cs).borrow_mut();
            let _ = serial.write(&bytes[0..len]);
        });
    }
}

impl UsbSerial {
    pub fn init(
        clocks: &mut GenericClockController,
        usb: pac::USB,
        pm: &mut pac::PM,
        dm: impl Into<UsbDm>,
        dp: impl Into<UsbDp>,
        nvic: &mut NVIC,
    ) -> UsbSerial {
        interrupt_cs::free(|_| {
            let bus_allocator = unsafe {
                BUS_ALLOCATOR = Some(xiao_m0::usb_allocator(
                    usb, clocks, pm, //&mut peripherals.PM,
                    dm, //pins.usb_dm,
                    dp, // pins.usb_dp,
                ));
                BUS_ALLOCATOR.as_mut().unwrap()
            };

            let serial = Mutex::new(RefCell::new(SerialPort::new(bus_allocator)));
            let bus = UsbDeviceBuilder::new(bus_allocator, UsbVidPid(0x16c0, 0x27dd))
                .manufacturer("Halemba")
                .product("Serial port")
                .serial_number("TEST")
                .device_class(USB_CLASS_CDC)
                .build();
            unsafe {
                nvic.set_priority(pac::interrupt::USB, 2);
                NVIC::unmask(pac::interrupt::USB);
            };
            UsbSerial { bus, serial }
        })
    }
}
