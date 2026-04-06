//! avr_core atmega128a

pub mod assembler;
pub mod cpu;
pub mod intel_hex;
pub mod io_map;
pub use cpu::Cpu;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum McuModel {
    Atmega128A,
    Atmega328P,
}

impl McuModel {
    pub fn label(self) -> &'static str {
        match self {
            Self::Atmega128A => "ATmega128A",
            Self::Atmega328P => "ATmega328P",
        }
    }

    pub fn flash_word_count(self) -> usize {
        match self {
            Self::Atmega128A => crate::avr::cpu::FLASH_WORDS_128A,
            Self::Atmega328P => crate::avr::cpu::FLASH_WORDS_328P,
        }
    }

    /// `avrdude -p` part id (short form).
    pub fn avrdude_part(self) -> &'static str {
        match self {
            Self::Atmega128A => "m128",
            Self::Atmega328P => "m328p",
        }
    }
}
