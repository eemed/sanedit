#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum Border {
    Box,
    Margin,
}

impl Border {
    pub fn top_left(&self) -> &str {
        match self {
            Border::Box => "┌",
            Border::Margin => " ",
        }
    }

    pub fn top_right(&self) -> &str {
        match self {
            Border::Box => "┐",
            Border::Margin => " ",
        }
    }

    pub fn bottom_right(&self) -> &str {
        match self {
            Border::Box => "┘",
            Border::Margin => " ",
        }
    }

    pub fn bottom_left(&self) -> &str {
        match self {
            Border::Box => "└",
            Border::Margin => " ",
        }
    }

    pub fn bottom(&self) -> &str {
        match self {
            Border::Box => "─",
            Border::Margin => " ",
        }
    }

    pub fn top(&self) -> &str {
        match self {
            Border::Box => "─",
            Border::Margin => " ",
        }
    }

    pub fn left(&self) -> &str {
        match self {
            Border::Box => "│",
            Border::Margin => " ",
        }
    }

    pub fn right(&self) -> &str {
        match self {
            Border::Box => "│",
            Border::Margin => " ",
        }
    }
}
