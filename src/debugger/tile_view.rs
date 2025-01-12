use std::time::Duration;

use sdl2::event::Event;
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::Canvas;

use crate::arm7tdmi::bus::Bus;
use crate::gba::GameBoyAdvance;
use crate::ioregs::consts::*;
use crate::lcd::*;
use crate::palette::*;
use crate::sysbus::SysBus;

impl Into<Color> for Rgb15 {
    fn into(self) -> Color {
        let (r, g, b) = self.get_rgb24();
        Color::RGB(r, g, b)
    }
}

fn draw_tile(
    gba: &GameBoyAdvance,
    tile_addr: u32,
    pixel_format: PixelFormat,
    p: Point,
    canvas: &mut Canvas<sdl2::video::Window>,
) {
    for y in 0..8 {
        for x in 0..8 {
            let color = match pixel_format {
                PixelFormat::BPP4 => {
                    let index =
                        gba.lcd
                            .read_pixel_index(&gba.sysbus, tile_addr, x, y, 4, pixel_format);
                    gba.lcd.get_palette_color(&gba.sysbus, index as u32, 0)
                }
                PixelFormat::BPP8 => {
                    let index =
                        gba.lcd
                            .read_pixel_index(&gba.sysbus, tile_addr, x, y, 8, pixel_format);
                    gba.lcd.get_palette_color(&gba.sysbus, index as u32, 0)
                }
            };
            let (r, g, b) = color.get_rgb24();

            canvas.set_draw_color(Color::RGB(r, g, b));
            canvas.draw_point(p.offset(x as i32, y as i32));
        }
    }
}

const TILESET_INITIAL_X: i32 = 0x20;
const TILESET_INITIAL_Y: i32 = 0x20;

pub fn create_tile_view(bg: u32, gba: &GameBoyAdvance) {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window("PaletteView", 512, 512)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();

    let bgcnt = BgControl::from(gba.sysbus.ioregs.read_reg(REG_BG0CNT + 2 * bg));

    let palette = Palette::from(gba.sysbus.get_bytes(0x0500_0000));
    let (tile_size, pixel_format) = bgcnt.tile_format();
    let tileset_addr = bgcnt.char_block();
    let tilemap_addr = bgcnt.screen_block();
    let tiles_per_row = 32;
    let num_tiles = 0x4000 / tile_size;
    println!("tileset: {:#x}, tilemap: {:#x}", tileset_addr, tilemap_addr);

    let mut event_pump = sdl_context.event_pump().unwrap();
    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'running,
                Event::MouseButtonDown { x, y, .. } => {
                    let click_point = Point::new(x, y);
                    let mut tile_x = TILESET_INITIAL_X;
                    let mut tile_y = TILESET_INITIAL_Y;
                    for t in 0..num_tiles {
                        let tile_addr = tileset_addr + t * tile_size;
                        if t != 0 && t % tiles_per_row == 0 {
                            tile_y += 10;
                            tile_x = TILESET_INITIAL_Y;
                        }
                        tile_x += 10;
                        if Rect::new(tile_x, tile_y, 8, 8).contains_point(click_point) {
                            println!("tile #{:#x}, addr={:#x}", t, tile_addr);
                        }
                    }
                }
                _ => {}
            }
        }


        canvas.set_draw_color(Color::RGB(00, 00, 00));
        canvas.clear();

        let mut tile_x = TILESET_INITIAL_X;
        let mut tile_y = TILESET_INITIAL_Y;
        for t in 0..num_tiles {
            let tile_addr = tileset_addr + t * tile_size;
            if t != 0 && t % tiles_per_row == 0 {
                tile_y += 10;
                tile_x = TILESET_INITIAL_Y;
            }
            tile_x += 10;
            draw_tile(
                gba,
                tile_addr,
                pixel_format,
                Point::from((tile_x, tile_y)),
                &mut canvas,
            );
        }
        canvas.present();
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
}
