use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub enum Redraw {
    Window(Vec<Vec<String>>),
}
