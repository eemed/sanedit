use sanedit_buffer::piece_tree::Mark;

pub struct Locations(Vec<Location>);

pub struct Location {
    mark: Mark,
    message: String,
}
