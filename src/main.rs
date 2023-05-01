use minifb::{Key, Window, WindowOptions, Scale, ScaleMode};
use std::fs::File;
use std::io::Read;
use std::env;
use std::thread;
use std::time::Duration;
use console::Term;
use rand::prelude::*;
use std::string::String;
mod timer;



const KEY_MAP: [(u8, Key); 16] = [
    (0x0, Key::X),
    (0x1, Key::Key1),
    (0x2, Key::Key2),
    (0x3, Key::Key3),
    (0x4, Key::Q),
    (0x5, Key::W),
    (0x6, Key::E),
    (0x7, Key::A),
    (0x8, Key::S),
    (0x9, Key::D),
    (0xA, Key::Z),
    (0xB, Key::C),
    (0xC, Key::Key4),
    (0xD, Key::R),
    (0xE, Key::F),
    (0xF, Key::V),
    ];


pub const FONT_SET: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0,		// 0
    0x20, 0x60, 0x20, 0x20, 0x70,		// 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0,		// 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0,		// 3
    0x90, 0x90, 0xF0, 0x10, 0x10,		// 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0,		// 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0,		// 6
    0xF0, 0x10, 0x20, 0x40, 0x40,		// 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0,		// 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0,		// 9
    0xF0, 0x90, 0xF0, 0x90, 0x90,		// A
    0xE0, 0x90, 0xE0, 0x90, 0xE0,		// B
    0xF0, 0x80, 0x80, 0x80, 0xF0,		// C
    0xE0, 0x90, 0x90, 0x90, 0xE0,		// D
    0xF0, 0x80, 0xF0, 0x80, 0xF0,		// E
    0xF0, 0x80, 0xF0, 0x80, 0x80		// F
];

pub struct Cpu {
    // index register
    pub i: u16,
    // program counter
    pub pc: u16,
    // memory
    pub memory: [u8; 4096],
    // registers
    pub v: [u8; 16],
    // peripherals
    pub display: Window,
    pub d_buffer: Vec<u32>,
    // stack
    pub stack: [u16; 16],
    // stack pointer
    pub sp: u8,
    // delay timer
    pub delay_timer: timer::DelayTimer,
    // sound timer
    pub sound_timer: u8,
    // rng
    pub rand: rand::rngs::ThreadRng,
    
    pub halt: bool,
    
    pub errors: String

}

fn read_word(memory: [u8; 4096], index: u16) -> u16 {
    (memory[index as usize] as u16) << 8
    | (memory[(index + 1) as usize] as u16)
}



impl Cpu {
    pub fn new() -> Cpu {
        Cpu {
            i: 0,
            pc: 0,
            memory: [0; 4096],
            v: [0; 16],
            display: Window::new(
                "CHIP-8 Emulator",
                64,
                32,
                WindowOptions {
                    borderless: false,
                    title: true,
                    resize: false,
                    scale: Scale::X16,
                    scale_mode: ScaleMode::Stretch,
                    topmost: true,
                    transparency: false,
                    none: false
                }
            ).unwrap_or_else(|e| panic!("{}", e)),
            d_buffer: vec![0; 64 * 32],
            stack: [0; 16],
            sp: 0,
            delay_timer: timer::DelayTimer::new(),
            sound_timer: 0,
            rand: rand::thread_rng(),
            halt: false,
            errors: String::from("")
        }
    }

    pub fn reset(&mut self) {
        self.i = 0;
        self.pc = 0x200;
        self.memory = [0; 4096];
        self.v = [0; 16];
        self.stack = [0; 16];
        self.d_buffer = vec![0; 64 * 32];
        self.sp = 0;
        self.delay_timer.set_value(0);
        self.sound_timer = 0;
        self.rand = rand::thread_rng();
        for i in 0..80 {
            self.memory[i] = FONT_SET[i];
        }
        self.halt = false;
    }

    pub fn stack_push(&mut self, value: u16) {
        self.stack[self.sp as usize] = value;
        self.sp += 1;
    }

    pub fn stack_pop(&mut self) -> u16 {
        self.sp -= 1;
        let value = self.stack[self.sp as usize];
        return value;
    }

    pub fn execute_cycle(&mut self) {
        let opcode: u16 = read_word(self.memory, self.pc);
        self.process_opcode(opcode);
        self.display();
    }

    pub fn is_pressed(&mut self, key: u8) -> bool {
        if self.display.is_key_down(KEY_MAP[key as usize].1){
            return true;
        }
        return false;
    }

    fn display(&mut self) {
        self.display.update_with_buffer(&self.d_buffer, 64, 32).unwrap();

        self.display.update();
        if self.display.is_key_down(Key::Escape){
            self.halt = true;
        }
    }

    fn wait_for_key_press(&mut self, reg_x: usize) {
        // Loop until a key is pressed
        loop {
            // Poll the window events to check for key presses
            self.display.update();
            // Check if any of the keys corresponding to registers 0-F are pressed
            for (key, en) in KEY_MAP.iter() {
                if self.display.is_key_down(*en) {
                    // If a key is pressed, store its value in the register and exit the loop
                    self.v[reg_x] = *key;
                    return;
                }
            }

            // If no key is pressed, wait for a short time to avoid busy-waiting
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }



    fn draw_sprite(&mut self, x: u8, y: u8, height: u8) {
        let sprite_addr = self.i as usize;

        for row in 0..height {
            let display_row = (y + row) as usize * 64;

            let sprite_row = self.memory[sprite_addr + row as usize] as u16;

            for col in 0..8 {
                let sprite_color = (sprite_row >> (7 - col)) & 0x1;

                let display_index = (x + col as u8) as usize + display_row;

                let display_color = if sprite_color == 1 { 0xffffff } else { 0x0 };
                if display_index < 64 * 32 {
                    self.d_buffer[display_index] ^= display_color;
                }
                if display_color == 0xffffff && self.d_buffer[display_index] == 0 {
                    self.v[0xF] = 1;
                }
            }
        }
    }

    fn process_opcode(&mut self, opcode: u16) {
        // extract various opcode parameters
        let x = ((opcode & 0x0F00) >> 8) as usize;
        let y = ((opcode & 0x00F0) >> 4) as usize;
        let vx = self.v[x];
        let vy = self.v[y];
        let nnn = opcode & 0x0FFF;
        let kk = (opcode & 0x00FF) as u8;
        let n = (opcode & 0x000F) as u8;
        
        match opcode >> 12 {
            0x0 => match opcode {
                0x00E0 => self.d_buffer = vec![0; 64 * 32],
                0x00EE => {
                    let addr = self.stack_pop();
                    self.pc = addr;
                }
                0x0 => (),
                _ => self.errors.push_str(format!("{:#06X}: Unknown opcode - {:#04X}\n", self.i, opcode).as_str())
            },
            0x1 => {
                self.pc = nnn;
                return;
            },
            0x2 => {
                self.stack_push(self.pc);
                self.pc = nnn;
                return;
            }
            0x3 => {
                if vx == kk {
                    self.pc += 2;
                }
            }
            0x4 => {
                if vx != kk {
                    self.pc += 2;
                }
            }
            0x5 => {
                if vx == kk {
                    self.pc +=2
                }
            }
            0x6 => self.v[x] = kk,
            0x7 => self.v[x] = vx.wrapping_add(kk),
            0x8 => match opcode & 0x000F {
                0x0 => self.v[x] = vy,
                0x1 => self.v[x] |= vy,
                0x2 => self.v[x] &= vy,
                0x3 => self.v[x] ^= vy,
                0x4 => {
                    let (result, overflow) = vx.overflowing_add(vy);
                    self.v[x] = result;
                    self.v[0xF] = overflow as u8;
                }
                0x5 => {
                    let (result, overflow) = vx.overflowing_sub(vy);
                    self.v[x] = result;
                    self.v[0xF] = overflow as u8;
                }
                0x6 => {
                    self.v[0xF] = vx & 0x1;
                    self.v[x] >>= 1;
                }
                0x7 => {
                    let (result, overflow) = vy.overflowing_sub(vx);
                    self.v[x] = result;
                    self.v[0xF] = overflow as u8;
                }
                0xE => {
                    self.v[0xF] = (vx >> 7) & 0x1;
                    self.v[x] <<= 1;
                }
                _ => self.errors.push_str(format!("{:#06X}: Unknown opcode - {:#04X}\n", self.i, opcode).as_str())
            }
            0x9 => {
                if vx != vy {
                    self.pc +=2
                }
            }
            0xA => self.i = nnn,
            0xB => {
                self.pc = nnn + self.v[0] as u16;
                return;
            },
            0xC => {
                let rng: u8 = self.rand.gen();
                self.v[x] &= rng;
            },
            0xD => self.draw_sprite(vx, vy, n),
            0xE => match kk {
                0x9E => {
                    if self.is_pressed(vx) {
                        self.pc += 2;
                    }
                }
                0xA1 => {
                    if !self.is_pressed(vx) {
                        self.pc += 2;
                    }
                }
                _ => self.errors.push_str(format!("{:#06X}: Unknown opcode - {:#04X}\n", self.i, opcode).as_str())
            }
            0xF => match kk {
                0x07 => self.v[x] = self.delay_timer.get_value(),
                0x0A => self.wait_for_key_press(x),
                0x15 => self.delay_timer.set_value(vx),
                0x18 => self.errors.push_str(format!("{:#06X}: Sound not implemented\n", self.i).as_str()),
                0x1E => self.i += vx as u16,
                0x29 => self.i = vx as u16 * 8 ,
                0x33 => {
                    self.memory[self.i as usize] = vx / 100;
                    self.memory[self.i as usize + 1] = (vx / 10) % 10;
                    self.memory[self.i as usize + 2] = vx % 10;
                }
                0x55 => {
                    for i in 0..=x {
                        self.memory[self.i as usize + i] = self.v[i];
                    }
                    self.i += x as u16;
                }
                0x65 => {
                    for i in 0..=x {
                        self.v[i] = self.memory[self.i as usize + i];
                    }
                    self.i += x as u16;
                }
                _ => self.errors.push_str(format!("{:#06X}: Unknown opcode - {:#04X}\n", self.i, opcode).as_str())
            },
            _ => self.errors.push_str(format!("{:#06X}: Unknown opcode - {:#04X}\n", self.i, opcode).as_str())
        }

        // increment the program counter
        self.pc += 2;
    }
    
    pub fn print_cpu_state(&mut self) {
        // Move the cursor to the top left corner of the terminal
        if cfg!(windows) {
            Term::stdout().clear_screen().unwrap();
        } else {
            print!("\x1B[2J\x1B[1;1H");
        }

        // Print the state of the processor
        println!("===== CPU STATE =====");
        println!("Program Counter: {:#06X}", self.pc);
        println!("Index Register: {:#06X}", self.i);
        println!("Opcode: {:#04X}", read_word(self.memory, self.pc));
        println!("Registers: ");
        for i in 0..16 {
            print!("V{:X}: {:#04X}\t", i, self.v[i]);
            if (i + 1) % 4 == 0 {
                println!("");
            }
        }
        println!("Stack Pointer: {}", self.sp);
        println!("Stack: ");
        for i in 0..self.sp as usize{
            print!("{:#06X}\t", self.stack[i]);
            if (i + 1) % 4 == 0 {
                println!("");
            }
        }
        println!("");
        println!("Delay Timer: {}", self.delay_timer.get_value());
        println!("Sound Timer: {}", self.sound_timer);
        println!("======= Error =======");
        Term::stdout().write_line(self.errors.as_str()).unwrap();
        Term::stdout().flush().unwrap();
    }
}

fn load_rom(cpu: &mut Cpu, path: &str) -> std::io::Result<()> {
    let mut rom_file = File::open(path)?;

    let mut buffer = Vec::new();
    rom_file.read_to_end(&mut buffer)?;

    let start_address = 0x200;
    for (i, &byte) in buffer.iter().enumerate() {
        let address = start_address + i as u16;
        cpu.memory[address as usize] = byte;
    }

    cpu.pc = start_address;

    Ok(())
}

fn main() {
    let debug = false;
    let mut processor: Cpu = Cpu::new();
    let args: Vec<String> = env::args().collect();
    processor.delay_timer.start();
    processor.reset();
    load_rom(&mut processor, args.get(1).unwrap()).unwrap_or_else(|e| panic!("{}", e));
    
    loop {
        processor.print_cpu_state();
        for _ in 0..1200 {
            processor.execute_cycle();
            thread::sleep(Duration::from_micros(100));
        }
        if processor.halt{
            break;
        }
        if debug {
            processor.wait_for_key_press(0xC);
        }
        
    }
    
}
