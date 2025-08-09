use std::ops::{Index, IndexMut};
use fastrand::u8;
use crate::chip8::Error::*;
use crate::chip8::Instruction::*;
use crate::chip8::RegOrVal::*;
use crate::{HEIGHT, WIDTH};

type RegisterIndex = u8;
type U12 = u16;

#[derive(Debug)]
enum RegOrVal {
    Value(U12),
    Register(RegisterIndex)
}

const DISPLAY_WIDTH_BYTES: usize = 8;
const DISPLAY_HEIGHT_BITS: usize = 32;

pub(crate) struct Options {
    late_shift: bool,
    late_jump_with_offset: bool,
    late_store_load: bool,
    vertical_wrap: bool,
    horizontal_wrap: bool,
}

pub(crate) struct Chip8 {
    pub(crate) memory: [u8; 4096],
    pub(crate) program_counter: U12,
    pub(crate) index_register: U12,
    pub(crate) stack: Vec<u16>,
    pub(crate) delay_timer: u8,
    pub(crate) sound_timer: u8,
    pub(crate) register: [u8; 16],
    pub(crate) screen: [[u8; DISPLAY_WIDTH_BYTES]; DISPLAY_HEIGHT_BITS],
    pub(crate) options: Options,
    pub(crate) keys: u16,
}

const FONT_START: usize = 0x50;
pub(crate) const INSTRUCTION_START: usize = 0x200;

impl Chip8 {
    pub fn new() -> Chip8 {
        Chip8{
            memory: [0;4096],
            program_counter: 0x200,
            index_register: 0,
            stack: vec![],
            delay_timer: 0,
            sound_timer: 0,
            register: [0;16],
            options: Options {
                late_shift: true,
                late_jump_with_offset: false,
                late_store_load: true,
                vertical_wrap: false,
                horizontal_wrap: false,
            },
            screen: [[0;DISPLAY_WIDTH_BYTES];DISPLAY_HEIGHT_BITS],
            keys: 0,
        }
    }

    pub fn tick(&mut self) -> Result<(), Error> {
        self.next().ok_or(EmptyFetch)?
            .fetch()?
            .execute(self)
    }

    pub fn get_lowest_key(&mut self) -> u8 {
        if self.keys != 0 {
            for i in 0..16u8 {
                if !bitu16(self.keys, i) {
                    return i
                }
            }
            unreachable!("Key is held and not held");
        }
        0
    }

    pub fn get_screen(&self, x: usize, y: usize) -> Result<bool, Error> {
        if x >= WIDTH {return Err(GetScreenOutOfBounds)}
        if y >= HEIGHT {return Err(GetScreenOutOfBounds)}
        Ok(self.screen[y][x/8] & 128u8 >> (x % 8) != 0)
    }

    pub fn set_screen(&mut self, mut x: usize, mut y: usize, value: bool) -> Result<(), Error> {
        if x >= WIDTH {
            if self.options.horizontal_wrap {
                x %= WIDTH;
            }
            else { return Err(SetScreenOutOfBounds) }
        }
        if y >= HEIGHT {
            if self.options.vertical_wrap {
                y %= HEIGHT;
            }
            else { return Err(SetScreenOutOfBounds) }
        }
        
        if value { self.screen[y][x/8] |= 128 >> (x % 8); }
        else { self.screen[y][x/8] &= !(128 >> (x % 8)); }
        Ok(())
    }

    pub fn load_font(&mut self) {
        for i in 0..80 {
            self.memory[i+FONT_START] =
                [0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
                    0x20, 0x60, 0x20, 0x20, 0x70, // 1
                    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
                    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
                    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
                    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
                    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
                    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
                    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
                    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
                    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
                    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
                    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
                    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
                    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
                    0xF0, 0x80, 0xF0, 0x80, 0x80  // F
                ][i]
        }
    }
}

#[derive(Debug)]
pub(crate) struct CodedInstruction (u16);

#[derive(Debug)]
enum Instruction {
    ClearScreen,
    Jump(U12),
    Subroutine(U12),
    SubroutineReturn,
    SkipIfEqual(RegisterIndex, RegOrVal),
    SkipIfNotEqual(RegisterIndex, RegOrVal),
    Set(RegisterIndex, RegOrVal),
    Add(RegisterIndex, u8),
    BinaryOR(RegisterIndex, RegisterIndex),
    BinaryAND(RegisterIndex, RegisterIndex),
    LogicalXOR(RegisterIndex, RegisterIndex),
    AddCarry(RegisterIndex, RegisterIndex),
    Subtract(RegisterIndex, RegisterIndex),
    SubtractNegative(RegisterIndex, RegisterIndex),
    ShiftLeft(RegisterIndex, RegisterIndex),
    ShiftRight(RegisterIndex, RegisterIndex),
    SetIndex(U12),
    JumpWithOffset(U12),
    Random(RegisterIndex, u8),
    Display(RegisterIndex, RegisterIndex, u8),
    SkipIfKeyPressed(RegisterIndex),
    SkipIfKeyNotPressed(RegisterIndex),
    SetToDelay(RegisterIndex),
    SetDelayTimer(RegisterIndex),
    SetSoundTimer(RegisterIndex),
    AddToIndex(RegisterIndex),
    GetKey(RegisterIndex),
    FontChar(RegisterIndex),
    DecimalConversion(RegisterIndex),
    StoreMemory(RegisterIndex),
    LoadMemory(RegisterIndex),
}

impl CodedInstruction {
    fn fetch(self) -> Result<Instruction, Error> {
        let (a,b,c,d) = (nibble(self.0, 0), nibble(self.0,1), nibble(self.0,2), nibble(self.0,3));
        match (a,b,c,d) {
            (0x0, 0x0, 0xE, 0x0)    => Ok( ClearScreen ),
            (0x0, 0x0, 0xE, 0xE)    => Ok( SubroutineReturn ),
            (0x1, _, _, _)          => Ok( Jump(self.0 & 0x0FFF) ),
            (0x2, _, _, _)          => Ok( Subroutine(self.0 & 0x0FFF) ),
            (0x3, _, _, _)          => Ok( SkipIfEqual(b, Value(self.0 & 0x00FF)) ),
            (0x4, _, _, _)          => Ok( SkipIfNotEqual(b, Value(self.0 & 0x00FF)) ),
            (0x5, _, _, _)          => Ok( SkipIfEqual(b, Register(c)) ),
            (0x6, _, _, _)          => Ok( Set(b, Value(self.0 & 0x00FF)) ),
            (0x7, _, _, _)          => Ok( Add(b, (self.0 & 0x00FF) as u8) ),
            (0x8, _, _, 0x0)        => Ok( Set(b, Register(c)) ),
            (0x8, _, _, 0x1)        => Ok( BinaryOR(b, c) ),
            (0x8, _, _, 0x2)        => Ok( BinaryAND(b, c) ),
            (0x8, _, _, 0x3)        => Ok( LogicalXOR(b, c) ),
            (0x8, _, _, 0x4)        => Ok( AddCarry(b,c) ),
            (0x8, _, _, 0x5)        => Ok( Subtract(b,c) ),
            (0x8, _, _, 0x6)        => Ok( ShiftRight(b,c) ),
            (0x8, _, _, 0x7)        => Ok( SubtractNegative(b,c) ),
            (0x8, _, _, 0xE)        => Ok( ShiftLeft(b,c) ),
            (0x9, _, _, 0x0)        => Ok( SkipIfNotEqual(b, Register(c)) ),
            (0xA, _, _, _)          => Ok( SetIndex(self.0 & 0x0FFF) ),
            (0xB, _, _, _)          => Ok( JumpWithOffset(self.0 & 0x0FFF) ),
            (0xC, _, _, _)          => Ok( Random(b, (self.0 & 0x00FF) as u8) ),
            (0xD, _, _, _)          => Ok( Display(b,c,d) ),
            (0xE, _, 0x9, 0xE)      => Ok( SkipIfKeyPressed(b) ),
            (0xE, _, 0xA, 0x1)      => Ok( SkipIfKeyNotPressed(b) ),
            (0xF, _, 0x0, 0x7)      => Ok( SetToDelay(b) ),
            (0xF, _, 0x1, 0x5)      => Ok( SetDelayTimer(b) ),
            (0xF, _, 0x1, 0x8)      => Ok( SetSoundTimer(b) ),
            (0xF, _, 0x1, 0xE)      => Ok( AddToIndex(b) ),
            (0xF, _, 0x0, 0xA)      => Ok( GetKey(b) ),
            (0xF, _, 0x2, 0x9)      => Ok( FontChar(b) ),
            (0xF, _, 0x3, 0x3)      => Ok( DecimalConversion(b) ),
            (0xF, _, 0x5, 0x5)      => Ok( StoreMemory(b) ),
            (0xF, _, 0x6, 0x5)      => Ok( LoadMemory(b) ),

            _ => Err(UnknownInstruction(self)),
        }
    }
}

impl Instruction {
    fn execute(self, chip8: &mut Chip8) -> Result<(), Error> {
        match self {
            ClearScreen => {
                chip8.screen = [[0; DISPLAY_WIDTH_BYTES]; DISPLAY_HEIGHT_BITS];
            }
            Jump(v) => {
                chip8.program_counter = v;
            }
            Subroutine(v) => {
                chip8.stack.push(chip8.program_counter as u16);
                chip8.program_counter = v;
            }
            SubroutineReturn => {
                let Some(s) = chip8.stack.pop() else { return Err(EmptySubroutineReturn) };
                chip8.program_counter = s;
            }
            SkipIfEqual(a, b) => {
                match b {
                    Value(v) => {if chip8[a] == v as u8 { chip8.program_counter += 2; } }
                    Register(r) => {if chip8[a] == chip8[r] { chip8.program_counter += 2; } }
                }
            }
            SkipIfNotEqual(a, b) => {
                match b {
                    Value(v) => {if chip8[a] != v as u8 { chip8.program_counter += 2; } }
                    Register(r) => {if chip8[a] != chip8[r] { chip8.program_counter += 2; } }
                }
            }
            Set(r, v) => {
                match v {
                    Value(a) => {chip8[r] = a as u8;}
                    Register(a) => {chip8[r] = chip8[a];}
                }
            }
            Add(a, b) => {
                chip8[a] = chip8[a].wrapping_add(b);
            }
            BinaryOR(a, b) => {
                chip8[a] |= chip8[b];
            }
            BinaryAND(a, b) => {
                chip8[a] &= chip8[b];
            }
            LogicalXOR(a, b) => {
                chip8[a] ^= chip8[b];
            }
            AddCarry(a, b) => {
                let carry;
                (chip8[a], carry) = chip8[a].overflowing_add(chip8[b]);
                chip8[15] = carry as u8;
            }
            Subtract(a, b) => {
                let carry;
                (chip8[a], carry) = chip8[a].overflowing_sub(chip8[b]);
                chip8[15] = !carry as u8;
            }
            SubtractNegative(a, b) => {
                let carry;
                (chip8[a], carry) = chip8[b].overflowing_sub(chip8[a]);
                chip8[15] = !carry as u8;
            }
            ShiftLeft(a, b) => {
                if chip8.options.late_shift {
                    chip8[a] = chip8[b];
                }
                let first = bitu8(chip8[a], 0);
                chip8[a] <<= 1;
                chip8[15] = first as u8;
            }
            ShiftRight(a, b) => {
                if chip8.options.late_shift {
                    chip8[a] = chip8[b];
                }
                let last = bitu8(chip8[a],7);
                chip8[a] >>= 1;
                chip8[15] = last as u8;
            }
            SetIndex(s) => {
                chip8.index_register = s;
            }
            JumpWithOffset(s) => {
                chip8.program_counter = s;
                if chip8.options.late_jump_with_offset {
                    chip8.program_counter += chip8[nibble(s,1)] as u16
                }
                else {
                    chip8.program_counter += chip8[0] as u16
                }
            }
            Random(r, v) => {
                chip8[r] = u8(..) & v
            }
            Display(x, y, n) => {
                let n = n as usize;
                let x = (chip8[x] & (DISPLAY_WIDTH_BYTES as u8 * 8 - 1)) as usize;
                let y = (chip8[y] & (DISPLAY_HEIGHT_BITS as u8 - 1)) as usize;
                chip8[0xF] = 0;
                for i in 0..n {
                    for j in 0..8usize {
                        if bitu8(chip8.memory[chip8.index_register as usize + i], j as u8) {
                            match chip8.get_screen(j + x, i + y) {
                                Ok(true) => {
                                    chip8[0xF] = 1;
                                    chip8.set_screen(j + x, i + y, false)?
                                }
                                Ok(false) => {
                                    chip8.set_screen(j + x, i + y, true)?
                                }
                                Err(_) => {}
                            }
                        }
                    }
                }
            }
            SkipIfKeyPressed(r) => {
                if bitu16(chip8.keys, chip8[r]) {
                    chip8.program_counter += 2;
                }
            }
            SkipIfKeyNotPressed(r) => {
                if !bitu16(chip8.keys, chip8[r]) {
                    chip8.program_counter += 2;
                }
            }
            SetToDelay(r) => {
                chip8[r] = chip8.delay_timer;
            }
            SetDelayTimer(r) => {
                chip8.delay_timer = chip8[r];
            }
            SetSoundTimer(r) => {
                chip8.sound_timer = chip8[r];
            }
            AddToIndex(r) => {
                let carry;
                (chip8.index_register, carry) = chip8.index_register.overflowing_add(chip8[r] as u16);
                /*if carry { chip8[15] = 1 };*/
            }
            GetKey(r) => {
                if chip8.keys == 0 { chip8.program_counter -= 2 }
                else { chip8[r] = chip8.get_lowest_key() }
            }
            FontChar(r) => {
                chip8.index_register = (chip8[r] * 5) as U12 + FONT_START as U12;
            }
            DecimalConversion(r) => {
                let a = chip8[r];
                chip8.memory[(chip8.index_register + 2) as usize] = a%10;
                chip8.memory[(chip8.index_register + 1) as usize] = a/10 % 10;
                chip8.memory[(chip8.index_register + 0) as usize] = a/100 % 10;
            }
            StoreMemory(r) => {
                if r > 15 {return Err(IllegalSave)}
                for i in 0..=r {
                    chip8.memory[chip8.index_register as usize + i as usize] = chip8[i];
                }
                if chip8.options.late_store_load {chip8.index_register += (r + 1) as u16}
            }
            LoadMemory(r) => {
                if r > 15 {return Err(IllegalLoad)}
                for i in 0..=r {
                    chip8[i] = chip8.memory[chip8.index_register as usize + i as usize];
                }
                if chip8.options.late_store_load {chip8.index_register += (r + 1) as u16}
            }
        }
        Ok(())
    }
}

fn nibble(input: u16, n: u8) -> RegisterIndex {
    match n {
        0 => (input & 0xF000) >> 12,
        1 => (input & 0x0F00) >> 8,
        2 => (input & 0x00F0) >> 4,
        3 => input & 0x000F,
        _ => unreachable!("Illegal nibble input")
    }.try_into().unwrap()
}

fn bitu16(input: u16, n: u8) -> bool {
    input & (0b1000000000000000 >> n) != 0
}

fn bitu8(input: u8, n: u8) -> bool {
    input & (0b10000000u8 >> n) != 0
}

#[derive(Debug)]
pub(crate) enum Error {
    UnknownInstruction(CodedInstruction),
    EmptyFetch,
    EmptySubroutineReturn,
    GetScreenOutOfBounds,
    SetScreenOutOfBounds,
    IllegalSave,
    IllegalLoad,
}

impl Iterator for Chip8 {
    type Item = CodedInstruction;
    fn next(&mut self) -> Option<Self::Item> {
        self.program_counter += 2;
        if self.program_counter > 4096 {return None}
        Some(CodedInstruction(u16::from_be_bytes([self.memory[self.program_counter as usize-2], self.memory[self.program_counter as usize - 1]])))
    }
}

impl Index<RegisterIndex> for Chip8 {
    type Output = u8;

    fn index(&self, index: RegisterIndex) -> &Self::Output {
        &self.register[index as usize]
    }
}

impl IndexMut<RegisterIndex> for Chip8 {
    fn index_mut(&mut self, index: RegisterIndex) -> &mut Self::Output {
        &mut self.register[index as usize]
    }
}