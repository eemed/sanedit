use sanedit_buffer::piece_tree::Mark;

pub struct Locations(Vec<Location>);

pub struct Location {
    pos: usize,
    mark: Option<Mark>,

    message: String,
}
