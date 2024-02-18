use pest_derive::Parser;

pub(crate) mod json {
    use super::*;
    #[derive(Parser)]
    #[grammar = "grammars/json.pest"]
    pub struct JsonParser;
}
