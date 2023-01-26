use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq,Default)]
pub struct Point {
    pub x: usize,
    pub y: usize,
}
