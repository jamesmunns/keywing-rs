#![no_std]
#![no_main]

// Panic provider crate
use panic_persist;
use cortex_m;

// Used to set the program entry point
use cortex_m_rt::entry;

// Provides definitions for our development board
use nrf52840_hal::{
    prelude::*,
    target::{Peripherals, CorePeripherals},
    Timer,
    Rng,
    Spim,
    spim::{
        Pins as SpimPins,
        Frequency as SpimFrequency,
        MODE_0,
    },
    gpio::{
        p0::Parts as P0Parts,
        p1::Parts as P1Parts,
        Level,
    },
    Clocks,
};

use rtt_target::{
    rprint, rprintln,
    rtt_init_print,
};

use embedded_graphics::{
    fonts::{Font8x16, Text},
    pixelcolor::Rgb565,
    prelude::*,
    style::{TextStyle, TextStyleBuilder},
};

mod buffer;

use ili9341::{Ili9341, Orientation};

const TEXT_SAMPLE: &[&str] = &[
    "for x in 0..10 {",
    "  for y in 0..10 {",
    "    let rand: u16 = rng.random_u16();",
    "    buffy.iter_mut().for_each(|px| {",
    "      *px = swap(rand)",
    "    });",
    "    lcd.draw_raw(",
    "      32 * x,",
    "      24 * y,",
    "      (32 * (x + 1)) - 1,",
    "      (24 * (y + 1)) - 1,",
    "      &buffy,",
    "    ).unwrap();",
    "  }",
    "}",
];

const TEXT_SAMPLE2: &[&[(i32, Rgb565, &str)]] = &[
    // "for x in 0..10 {",
    &[(0, Rgb565::RED, "for "), (4, Rgb565::WHITE, "x "), (6, Rgb565::RED, "in "), (9, Rgb565::MAGENTA, "0"), (10, Rgb565::RED, ".."), (12, Rgb565::MAGENTA, "10"), (14, Rgb565::WHITE, " {")],
    // "  for y in 0..10 {",
    &[(2, Rgb565::RED, "for "), (6, Rgb565::WHITE, "y "), (8, Rgb565::RED, "in "), (11, Rgb565::MAGENTA, "0"), (12, Rgb565::RED, ".."), (14, Rgb565::MAGENTA, "10"), (16, Rgb565::WHITE, " {")],
    // "    let rand: u16 = rng.random_u16();",
    &[(4, Rgb565::CYAN, "let "), (8, Rgb565::WHITE, "rand: "), (14, Rgb565::CYAN, "u16 "), (18, Rgb565::RED, "= "), (20, Rgb565::WHITE, "rng."), (24, Rgb565::CYAN, "random_u16"), (34, Rgb565::WHITE, "();")],
    // "    buffy.iter_mut().for_each(|px| {",
    &[(4, Rgb565::WHITE, "buffy."), (10, Rgb565::CYAN, "iter_mut"), (18, Rgb565::WHITE, "()."), (21, Rgb565::CYAN, "for_each"), (29, Rgb565::WHITE, "(|"), (31, Rgb565::YELLOW, "px"), (33, Rgb565::WHITE, "| {")],
    // "      *px = swap(rand)",
    &[(6, Rgb565::RED, "*"), (7, Rgb565::WHITE, "px "), (10, Rgb565::RED, "= "), (12, Rgb565::CYAN, "swap"), (16, Rgb565::WHITE, "(rand)")],
    // "    });",
    &[(4, Rgb565::WHITE, "});")],
    // "    lcd.draw_raw(",
    &[(4, Rgb565::WHITE, "lcd."), (8, Rgb565::CYAN, "draw_raw"), (16, Rgb565::WHITE, "(")],
    // "      32 * x,",
    &[(6, Rgb565::MAGENTA, "32 "), (9, Rgb565::RED, "* "), (11, Rgb565::WHITE, "x,")],
    // "      24 * y,",
    &[(6, Rgb565::MAGENTA, "24 "), (9, Rgb565::RED, "* "), (11, Rgb565::WHITE, "y,")],
    // "      (32 * (x + 1)) - 1,",
    &[(6, Rgb565::WHITE, "("), (7, Rgb565::MAGENTA,"32 "), (10, Rgb565::RED, "* "), (12, Rgb565::WHITE, "(x "), (15, Rgb565::RED, "+ "), (17, Rgb565::MAGENTA, "1"), (18, Rgb565::WHITE, ")) "), (21, Rgb565::RED, "- "), (23, Rgb565::MAGENTA, "1"), (24, Rgb565::WHITE, ",")],
    // "      (24 * (y + 1)) - 1,",
    &[(6, Rgb565::WHITE, "("), (7, Rgb565::MAGENTA,"24 "), (10, Rgb565::RED, "* "), (12, Rgb565::WHITE, "(y "), (15, Rgb565::RED, "+ "), (17, Rgb565::MAGENTA, "1"), (18, Rgb565::WHITE, ")) "), (21, Rgb565::RED, "- "), (23, Rgb565::MAGENTA, "1"), (24, Rgb565::WHITE, ",")],
    // "      &buffy,",
    &[(6, Rgb565::RED, "&"), (7, Rgb565::WHITE, "buffy,")],
    // "    ).unwrap();",
    &[(4, Rgb565::WHITE, ")."), (6, Rgb565::CYAN, "unwrap"), (12, Rgb565::WHITE, "();")],
    // "  }",
    &[(2, Rgb565::WHITE, "}")],
    // "}",
    &[(0, Rgb565::WHITE, "}")],
];

#[entry]
fn main() -> ! {
    match inner_main() {
        Ok(()) => cortex_m::peripheral::SCB::sys_reset(),
        Err(e) => panic!(e),
    }
}

fn inner_main() -> Result<(), &'static str> {
    let mut board = Peripherals::take().ok_or("Error getting board!")?;
    let mut corep = CorePeripherals::take().ok_or("Error")?;
    let mut timer = Timer::new(board.TIMER0);
    let mut delay = Timer::new(board.TIMER1);
    let mut rng = Rng::new(board.RNG);
    let mut toggle = false;
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
    let stm_cs = p0.p0_07; // GPIO6, D6,
    let lcd_cs = p0.p0_26; // GPIO9, D9,
    let lcd_dc = p0.p0_27; // GPIO10, D10

    // Pull the neopixel line low so noise doesn't make it turn on spuriously
    let keywing_neopixel = p0.p0_06.into_push_pull_output(Level::Low); // GPIO11, D11

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
    ).unwrap();

    lcd.set_orientation(Orientation::Landscape).unwrap();

    let mut buffy = [0u16; 24 * 32];
    let mut buffy2 = [[0u16; 320]; 240];

    let mut fbuffy = buffer::FrameBuffer::new(&mut buffy2);

    // //                                     rrrrr gggggg bbbbb
    // buffy.iter_mut().for_each(|px| *px = 0b11111_000000_00000);

    let style = TextStyleBuilder::new(Font8x16)
        .text_color(Rgb565::WHITE)
        .background_color(Rgb565::BLACK)
        .build();

    loop {

        rprintln!("Start colors raw");

        for x in 0..10 {
            for y in 0..10 {
                let rand: u16 = rng.random_u16();
                buffy.iter_mut().for_each(|px| {
                    *px = swap(rand)
                });

                lcd.draw_raw(
                    32 * x,
                    24 * y,
                    (32 * (x + 1)) - 1,
                    (24 * (y + 1)) - 1,
                    &buffy,
                ).unwrap();
            }
        }

        rprintln!("Done.\n");

        timer.delay_ms(1000u16);

        rprintln!("Start colors raw");

        for x in 0..10 {
            for y in 0..10 {
                let rand: u16 = 0;
                buffy.iter_mut().for_each(|px| {
                    *px = swap(rand)
                });

                lcd.draw_raw(
                    32 * x,
                    24 * y,
                    (32 * (x + 1)) - 1,
                    (24 * (y + 1)) - 1,
                    &buffy,
                ).unwrap();
            }
        }

        rprintln!("Done.\n");


        timer.delay_ms(1000u16);

        // buffy2.iter_mut().for_each(|px| *px = swap(0b00000_000000_00000));
        // rprintln!("Start black");
        // lcd.draw_raw(0, 0, 319, 239, &buffy2).unwrap();
        // rprintln!("Done.\n");

        rprintln!("text start");

        for row in 0..15 {
            Text::new(
                TEXT_SAMPLE[row as usize],
                Point::new(0, row * 16)
            ).into_styled(style)
            .draw(&mut lcd)
            .unwrap();
        }

        rprintln!("text done");

        timer.delay_ms(3000u16);

        rprintln!("Start colors raw");

        for x in 0..10 {
            for y in 0..10 {
                let rand: u16 = 0;
                buffy.iter_mut().for_each(|px| {
                    *px = swap(rand)
                });

                lcd.draw_raw(
                    32 * x,
                    24 * y,
                    (32 * (x + 1)) - 1,
                    (24 * (y + 1)) - 1,
                    &buffy,
                ).unwrap();
            }
        }

        rprintln!("Done.\n");


        timer.delay_ms(1000u16);

        // buffy2.iter_mut().for_each(|px| *px = swap(0b00000_000000_00000));
        // rprintln!("Start black");
        // lcd.draw_raw(0, 0, 319, 239, &buffy2).unwrap();
        // rprintln!("Done.\n");

        rprintln!("text2 start");

        for (i, row) in TEXT_SAMPLE2.iter().enumerate() {
            for (offset, color, text) in row.iter() {
                let styled = TextStyleBuilder::new(Font8x16)
                    .text_color(*color)
                    .background_color(Rgb565::BLACK)
                    .build();

                Text::new(
                    text,
                    Point::new(*offset * 8, i as i32 * 16)
                ).into_styled(styled)
                .draw(&mut lcd)
                .unwrap();
            }
        }

        rprintln!("text2 done");

        timer.delay_ms(3000u16);

        rprintln!("Start colors raw");

        for x in 0..10 {
            for y in 0..10 {
                let rand: u16 = 0;
                buffy.iter_mut().for_each(|px| {
                    *px = swap(rand)
                });

                lcd.draw_raw(
                    32 * x,
                    24 * y,
                    (32 * (x + 1)) - 1,
                    (24 * (y + 1)) - 1,
                    &buffy,
                ).unwrap();
            }
        }

        rprintln!("Done.\n");

        timer.delay_ms(1000u16);

        // buffy2.iter_mut().for_each(|px| *px = swap(0b00000_000000_00000));
        // rprintln!("Start black");
        // lcd.draw_raw(0, 0, 319, 239, &buffy2).unwrap();
        // rprintln!("Done.\n");

        timer.start(1_000_000u32);

        let start: u32 = timer.read();

        for row in 0..15 {
            Text::new(
                TEXT_SAMPLE[row as usize],
                Point::new(0, row * 16)
            ).into_styled(style)
            .draw(&mut fbuffy)
            .unwrap();
        }

        let middle: u32 = timer.read();

        lcd.draw_raw(0, 0, 319, 239, fbuffy.inner()).unwrap();

        let end: u32 = timer.read();

        rprintln!("text buffered done");
        rprintln!("start: 0x{:08X}, middle: 0x{:08X}, end: 0x{:08X}", start, middle, end);
        rprintln!("render: {} cycs", middle - start);
        rprintln!("draw:   {} cycs", end - middle);





        timer.delay_ms(3000u16);

        rprintln!("Start colors raw");

        for x in 0..10 {
            for y in 0..10 {
                let rand: u16 = 0;
                buffy.iter_mut().for_each(|px| {
                    *px = swap(rand)
                });

                lcd.draw_raw(
                    32 * x,
                    24 * y,
                    (32 * (x + 1)) - 1,
                    (24 * (y + 1)) - 1,
                    &buffy,
                ).unwrap();
            }
        }

        rprintln!("Done.\n");


        timer.delay_ms(1000u16);

        rprintln!("text2 buffered middle");

        for (i, row) in TEXT_SAMPLE2.iter().enumerate() {
            for (offset, color, text) in row.iter() {
                let styled = TextStyleBuilder::new(Font8x16)
                    .text_color(*color)
                    .background_color(Rgb565::BLACK)
                    .build();

                Text::new(
                    text,
                    Point::new(*offset * 8, i as i32 * 16)
                ).into_styled(styled)
                .draw(&mut fbuffy)
                .unwrap();
            }
        }

        rprintln!("text2 buffered middle");

        lcd.draw_raw(0, 0, 319, 239, fbuffy.inner()).unwrap();

        rprintln!("text2 buffered done");

        timer.delay_ms(3000u16);

        continue;


        // // SHOULD BE
        // //                                      rrrrr gggggg bbbbb
        // buffy2.iter_mut().for_each(|px| *px = swap(0b11111_000000_00000));
        // rprintln!("Start red");
        // lcd.draw_raw(0, 0, 319, 239, &buffy2).unwrap();
        // rprintln!("Done.\n");

        // timer.delay_ms(250u16);

        // buffy2.iter_mut().for_each(|px| *px = swap(0b00000_111111_00000));
        // rprintln!("Start green");
        // lcd.draw_raw(0, 0, 319, 239, &buffy2).unwrap();
        // rprintln!("Done.\n");

        // timer.delay_ms(250u16);

        // buffy2.iter_mut().for_each(|px| *px = swap(0b00000_000000_11111));
        // rprintln!("Start blue");
        // lcd.draw_raw(0, 0, 319, 239, &buffy2).unwrap();
        // rprintln!("Done.\n");

        // buffy2.iter_mut().for_each(|px| *px = swap(0b00000_000000_00000));
        // rprintln!("Start black");
        // lcd.draw_raw(0, 0, 319, 239, &buffy2).unwrap();
        // rprintln!("Done.\n");

        // 240 / 16: 15
        // 320 /  8: 40

        let text = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890";

        let mut textiter = text.chars().cycle();

        for row in 0..15 {
            for col in 0..40 {
                let mut buf = [0u8; 4];
                let txt = textiter.next().unwrap().encode_utf8(&mut buf);
                Text::new(
                    txt,
                    Point::new(col * 8, row * 16)
                ).into_styled(style)
                .draw(&mut lcd)
                .unwrap();
            }
            timer.delay_ms(500u16);
        }

        timer.delay_ms(1000u16);


        // buffy2.iter_mut().for_each(|px| *px = swap(0b00000_000000_00000));
        // rprintln!("Start black");
        // lcd.draw_raw(0, 0, 319, 239, &buffy2).unwrap();
        // rprintln!("Done.\n");

        rprintln!("Starting Text Fill...");

        let mut text = Text::new(
            "1234567890123456789012345678901234567890",
            Point::new(0, 0)
        ).into_styled(style);

        for _y in 0..15 {
            text.draw(&mut lcd).unwrap();
            text = text.translate(Point::new(0, 16));
        }

        rprintln!("Finished Text Fill.");


        timer.delay_ms(1000u16);


    }

    Ok(())
}

const fn swap(inp: u16) -> u16 {
    (inp & 0x00FF) << 8 |
    (inp & 0xFF00) >> 8
}
