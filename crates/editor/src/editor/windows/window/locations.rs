#[derive(Debug)]
pub struct Locations {
    locations: Vec<Location>,
}

#[derive(Debug)]
pub enum Location {
    Group {
        name: String,
        locations: Vec<Location>,
    },
    Item {
        name: String,
        line: Option<usize>,
        column: Option<usize>,
    },
}
