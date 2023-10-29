pub struct Gutter {
    pub(crate) nums: Vec<usize>,
    pub(crate) width: usize,
}

impl Gutter {
    pub fn new(numbers: Vec<usize>) -> Gutter {
        let width = numbers
            .last()
            .map(|n| n.to_string().chars().count())
            .unwrap_or(0);

        Gutter {
            width,
            nums: numbers,
        }
    }

    pub fn width(&self) -> usize {
        self.width + 1
    }
}
