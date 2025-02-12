// https://en.wikipedia.org/wiki/Glob_(programming)
//
#[derive(Debug)]
pub struct Glob {}

impl Glob {
    pub fn new() -> Glob {
        todo!()
    }

    pub fn matches<B: AsRef<[u8]>>(bytes: &B) -> bool {
        let bytes = bytes.as_ref();

        false
    }
}
