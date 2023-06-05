use sanedit_buffer::Mark;

pub struct Locations(Vec<Location>);

pub struct Location {
    mark: Mark,
    message: String,
}
