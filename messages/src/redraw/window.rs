use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Window {
    cells: Vec<Vec<Cell>>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Cell {
    text: String,
}
