#![no_std]
#![no_main]

// Panic provider crate
use panic_persist as _;

// Used to set the program entry point
use cortex_m_rt::entry;

// Provides definitions for our development board
use nrf52840_hal::{
    gpio::{p0::Parts as P0Parts, p1::Parts as P1Parts, Level},
    prelude::*,
    spim::{Frequency as SpimFrequency, Pins as SpimPins, MODE_0},
    target::Peripherals,
    twim::{Frequency as TwimFrequency, Pins as TwimPins},
    Clocks, Rng, Spim, Timer, Twim,
};

use rtt_target::{rprintln, rtt_init_print};

use embedded_graphics::{
    fonts::{Font8x16, Text},
    pixelcolor::Rgb565,
    prelude::*,
    style::TextStyleBuilder,
};

use bbq10kbd::{Bbq10Kbd, KeyRaw};

mod buffer;

use ili9341::{Ili9341, Orientation};

#[entry]
fn main() -> ! {
    match inner_main() {
        Ok(()) => cortex_m::peripheral::SCB::sys_reset(),
        Err(e) => panic!(e),
    }
}

fn inner_main() -> Result<(), &'static str> {
    let board = Peripherals::take().ok_or("Error getting board!")?;
    let mut timer = Timer::new(board.TIMER0);
    let mut delay = Timer::new(board.TIMER1);
    let mut _rng = Rng::new(board.RNG);
    let _clocks = Clocks::new(board.CLOCK).enable_ext_hfosc();

    // use ChannelMode::NoBlockS
    rtt_init_print!(NoBlockSkip, 4096);

    if let Some(msg) = panic_persist::get_panic_message_utf8() {
        rprintln!("{}", msg);
    } else {
        rprintln!("Clean boot!");
    }

    let p0 = P0Parts::new(board.P0);
    let p1 = P1Parts::new(board.P1);

    let kbd_lcd_reset = p1.p1_08; // GPIO5, D5
    let _stm_cs = p0.p0_07; // GPIO6, D6,
    let lcd_cs = p0.p0_26; // GPIO9, D9,
    let lcd_dc = p0.p0_27; // GPIO10, D10

    let kbd_sda = p0.p0_12.into_floating_input().degrade();
    let kbd_scl = p0.p0_11.into_floating_input().degrade();

    let kbd_i2c = Twim::new(
        board.TWIM0,
        TwimPins {
            sda: kbd_sda,
            scl: kbd_scl,
        },
        TwimFrequency::K100,
    );

    let mut kbd = Bbq10Kbd::new(kbd_i2c);

    // Pull the neopixel lines low so noise doesn't make it turn on spuriously
    let _keywing_neopixel = p0.p0_06.into_push_pull_output(Level::Low); // GPIO11, D11
    let _feather_neopixel = p0.p0_16.into_push_pull_output(Level::Low);

    let spim = Spim::new(
        board.SPIM3,
        SpimPins {
            sck: p0.p0_14.into_push_pull_output(Level::Low).degrade(),
            miso: Some(p0.p0_15.into_floating_input().degrade()),
            mosi: Some(p0.p0_13.into_push_pull_output(Level::Low).degrade()),
        },
        SpimFrequency::M32,
        MODE_0,
        0x00,
    );

    let mut lcd = Ili9341::new_spi(
        spim,
        lcd_cs.into_push_pull_output(Level::High),
        lcd_dc.into_push_pull_output(Level::High),
        kbd_lcd_reset.into_push_pull_output(Level::High),
        &mut delay,
    )
    .unwrap();

    lcd.set_orientation(Orientation::Landscape).unwrap();

    let mut _buffy = [0u16; 24 * 32];
    let mut buffy2 = [[0u16; 320]; 240];

    let mut fbuffy = buffer::FrameBuffer::new(&mut buffy2);

    // //                                     rrrrr gggggg bbbbb
    // buffy.iter_mut().for_each(|px| *px = 0b11111_000000_00000);

    let mut style = TextStyleBuilder::new(Font8x16)
        .text_color(Rgb565::WHITE)
        .background_color(Rgb565::BLACK)
        .build();

    kbd.set_backlight(255).unwrap();

    let vers = kbd.get_version().unwrap();

    rprintln!("Vers: {:?}", vers);

    kbd.sw_reset().unwrap();
    timer.delay_ms(10u8);

    let vers = kbd.get_version().unwrap();

    rprintln!("Vers: {:?}", vers);

    let mut cursor = Cursor { x: 0, y: 0 };

    lcd.clear(Rgb565::BLACK).map_err(|_| "Fade to error")?;
    fbuffy.clear(Rgb565::BLACK).map_err(|_| "Fade to error")?;

    loop {
        let key = kbd.get_fifo_key_raw().map_err(|_| "bad fifo")?;

        match key {
            // LL
            KeyRaw::Pressed(6) => {
                style = TextStyleBuilder::new(Font8x16)
                    .text_color(Rgb565::WHITE)
                    .background_color(Rgb565::BLACK)
                    .build();
            }
            // LR
            KeyRaw::Pressed(17) => {
                style = TextStyleBuilder::new(Font8x16)
                    .text_color(Rgb565::RED)
                    .background_color(Rgb565::BLACK)
                    .build();
            }
            // RL
            KeyRaw::Pressed(7) => {
                style = TextStyleBuilder::new(Font8x16)
                    .text_color(Rgb565::GREEN)
                    .background_color(Rgb565::BLACK)
                    .build();
            }
            // RR
            KeyRaw::Pressed(18) => {
                style = TextStyleBuilder::new(Font8x16)
                    .text_color(Rgb565::BLUE)
                    .background_color(Rgb565::BLACK)
                    .build();
            }
            // Up
            KeyRaw::Pressed(1) => {
                cursor.up();
            }
            // Down
            KeyRaw::Pressed(2) => {
                cursor.down();
            }
            // Left
            KeyRaw::Pressed(3) => {
                cursor.left();
            }
            // Right
            KeyRaw::Pressed(4) => {
                cursor.right();
            }
            // Center
            KeyRaw::Pressed(5) => {
                kbd.sw_reset().unwrap();
                cursor = Cursor { x: 0, y: 0 };
                fbuffy.clear(Rgb565::BLACK).map_err(|_| "Fade to error")?;
            }
            // Backspace
            KeyRaw::Pressed(8) => {
                cursor.left();
                Text::new(" ", cursor.pos())
                    .into_styled(style)
                    .draw(&mut fbuffy)
                    .map_err(|_| "bad lcd")?;
            }
            // Enter
            KeyRaw::Pressed(10) => {
                cursor.enter();
            }
            KeyRaw::Pressed(k) => {
                rprintln!("Got key {}", k);
                if let Ok(s) = core::str::from_utf8(&[k]) {
                    Text::new(s, cursor.pos())
                        .into_styled(style)
                        .draw(&mut fbuffy)
                        .map_err(|_| "bad lcd")?;

                    cursor.right();
                }
            }
            KeyRaw::Invalid => {
                if let Some(buf) = fbuffy.inner() {
                    timer.start(1_000_000u32);
                    lcd.draw_raw(0, 0, 319, 239, buf).map_err(|_| "bad buffy")?;
                    let done = timer.read();
                    rprintln!("Drew in {}ms.", done / 1000);
                } else {
                    timer.delay_ms(38u8);
                }
            }
            _ => {}
        }
    }
}

struct Cursor {
    x: i32,
    y: i32,
}

impl Cursor {
    fn up(&mut self) {
        self.y -= 1;
        if self.y < 0 {
            self.y = 0;
        }
    }

    fn down(&mut self) {
        self.y += 1;
        if self.y >= 15 {
            self.y = 14;
        }
    }

    fn left(&mut self) {
        self.x -= 1;
        if self.x < 0 {
            if self.y != 0 {
                self.x = 39;
                self.up();
            } else {
                self.x = 0;
            }
        }
    }

    fn right(&mut self) {
        self.x += 1;
        if self.x >= 40 {
            self.x = 0;
            self.down();
        }
    }

    fn enter(&mut self) {
        if self.y != 14 {
            self.x = 0;
            self.down();
        }
    }

    fn pos(&self) -> Point {
        Point::new(self.x * 8, self.y * 16)
    }
}
