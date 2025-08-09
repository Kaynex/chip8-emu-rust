mod chip8;

use std::fs;
use minifb::{Key, Scale, Window, WindowOptions};
use crate::chip8::*;
use rfd::FileDialog;

pub(crate) const WIDTH: usize = 64;
pub(crate) const HEIGHT: usize = 32;
const TICKS_PER_FRAME: usize = 50;

fn main() {
    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];
    let mut emu: Chip8 = Chip8::new();
    emu.load_font();

    let rom = fs::read(FileDialog::new()
        .add_filter("", &["ch8"])
        .pick_file()
        .unwrap()
        .as_path())
        .unwrap();

    for (i,byte) in rom.iter().enumerate() {
        emu.memory[i + INSTRUCTION_START] = *byte;
    }

    let mut window = Window::new(
        "Test - ESC to exit",
        WIDTH,
        HEIGHT,
        WindowOptions {
            resize: true,
            scale: Scale::X16,
            ..WindowOptions::default()
        },
    )
        .unwrap_or_else(|e| {
            panic!("{}", e);
        });

    window.set_target_fps(60);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        assign_keys(&window, &mut emu);
        for _ in 0..TICKS_PER_FRAME {
            emu.tick().unwrap();
        }
        if emu.delay_timer > 0 {emu.delay_timer -= 1;}
        if emu.sound_timer > 0 {emu.sound_timer -= 1;}

        for x in 0..WIDTH {
            for y in 0..HEIGHT {
                if emu.get_screen(x,y).unwrap() { buffer[WIDTH*y + x] = 0x00FFFFFF }
                else { buffer[64*y + x] = 0x00000000 }
            }
        }
        window
            .update_with_buffer(&buffer, WIDTH, HEIGHT)
            .unwrap();
    }
}

fn assign_keys(window: &Window, emu: &mut Chip8) {
    emu.keys = 0;
    if window.is_key_released(Key::Key0) {emu.keys |= 0b1000000000000000};
    if window.is_key_released(Key::Key1) {emu.keys |= 0b0100000000000000};
    if window.is_key_released(Key::Key2) {emu.keys |= 0b0010000000000000};
    if window.is_key_released(Key::Key3) {emu.keys |= 0b0001000000000000};
    if window.is_key_released(Key::Key4) {emu.keys |= 0b0000100000000000};
    if window.is_key_released(Key::Key5) {emu.keys |= 0b0000010000000000};
    if window.is_key_released(Key::Key6) {emu.keys |= 0b0000001000000000};
    if window.is_key_released(Key::Key7) {emu.keys |= 0b0000000100000000};
    if window.is_key_released(Key::Key8) {emu.keys |= 0b0000000010000000};
    if window.is_key_released(Key::Key9) {emu.keys |= 0b0000000001000000};
    if window.is_key_released(Key::A)    {emu.keys |= 0b0000000000100000};
    if window.is_key_released(Key::B)    {emu.keys |= 0b0000000000010000};
    if window.is_key_released(Key::C)    {emu.keys |= 0b0000000000001000};
    if window.is_key_released(Key::D)    {emu.keys |= 0b0000000000000100};
    if window.is_key_released(Key::E)    {emu.keys |= 0b0000000000000010};
    if window.is_key_released(Key::F)    {emu.keys |= 0b0000000000000001};
}