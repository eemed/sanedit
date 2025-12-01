use core::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord, Hash)]
pub enum Color {
    Black,
    White,
    Rgb(Rgb),
}

impl Color {
    pub fn from_hex(string: &str) -> Result<Color, HexStringError> {
        Rgb::from_hex(string).map(Color::Rgb)
    }

    pub fn parse(string: &str) -> Result<Color, HexStringError> {
        match string {
            "black" => Ok(Color::Black),
            "white" => Ok(Color::White),
            _ => {
                let res = Rgb::from_hex(string).map(Color::Rgb);

                if res.is_err() {
                    if let Some(color) = Rgb::from_rgba(string).map(Color::Rgb) {
                        return Ok(color);
                    }
                }

                res
            }
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, PartialOrd, Ord, Hash)]
pub struct Rgb {
    pub(crate) red: u8,
    pub(crate) green: u8,
    pub(crate) blue: u8,
}

impl fmt::Debug for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Rgb")
            .field("red", &self.red)
            .field("green", &self.green)
            .field("blue", &self.blue)
            .field("(hex)", &self.name())
            .finish()
    }
}

impl Rgb {
    pub fn new(red: u8, green: u8, blue: u8) -> Rgb {
        Rgb { red, green, blue }
    }

    /// rgb(100,100,100)
    /// rgba(100,100,100,0.2)
    pub fn from_rgba(string: &str) -> Option<Rgb> {
        let bytes = string.as_bytes();
        let start = bytes.iter().position(|c| c.is_ascii_digit()).unwrap_or(0);
        let end = bytes
            .iter()
            .rposition(|c| c.is_ascii_digit())
            .map(|i| i + 1)
            .unwrap_or(string.len());
        let inside = &string[start..end];
        let mut splits = inside.split(",").map(str::trim);
        let red: u8 = splits.next()?.parse().ok()?;
        let green: u8 = splits.next()?.parse().ok()?;
        let blue: u8 = splits.next()?.parse().ok()?;
        if splits.next().is_some() {
            return None;
        }

        Some(Rgb::new(red, green, blue))
    }

    pub fn from_hex(string: &str) -> Result<Rgb, HexStringError> {
        // #123456 => 7 chars
        if string.len() != 7 {
            return Err(HexStringError::InvalidLength);
        }

        let mut chars = string.chars().enumerate();

        let first_char = chars.next().map(|s| s.1);
        if first_char != Some('#') {
            return Err(HexStringError::InvalidCharacter {
                pos: 0,
                found: first_char.unwrap(),
                expected: "#".to_string(),
            });
        }

        for (i, ch) in chars {
            match ch {
                '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' | 'a' | 'b' | 'c'
                | 'd' | 'e' | 'f' | 'A' | 'B' | 'C' | 'D' | 'E' | 'F' => {}
                _ => {
                    return Err(HexStringError::InvalidCharacter {
                        pos: i,
                        found: ch,
                        expected: "0-9 or A-F or a-f".to_string(),
                    });
                }
            }
        }

        let red = u8::from_str_radix(&string[1..3], 16).unwrap();
        let green = u8::from_str_radix(&string[3..5], 16).unwrap();
        let blue = u8::from_str_radix(&string[5..7], 16).unwrap();

        Ok(Rgb { red, green, blue })
    }

    pub fn name(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.red, self.green, self.blue)
    }

    pub fn get(&self) -> (u8, u8, u8) {
        (self.red, self.green, self.blue)
    }
}

#[derive(Error, Debug)]
pub enum HexStringError {
    #[error("Hex string has invalid length")]
    InvalidLength,

    #[error("Hex string has invalid character at pos {pos}, found {found}, expected {expected}")]
    InvalidCharacter {
        pos: usize,
        expected: String,
        found: char,
    },
}
