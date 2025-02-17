use crate::{
    grammar::{Rule, RuleInfo, Rules},
    Parser,
};

// https://en.wikipedia.org/wiki/Glob_(programming)
//
#[allow(dead_code)]
#[derive(Debug)]
pub struct Glob {
    parser: Parser,
}

#[allow(dead_code)]
impl Glob {
    pub fn new(pattern: &str) -> Glob {
        // Just testing here that this works OK, should probably do something better => just parse manually as this is prett simple
        let text = include_str!("../pegs/glob.peg");
        let parser = Parser::new(std::io::Cursor::new(text)).unwrap();
        let captures = parser.parse(pattern).unwrap();
        let mut rules: Vec<Rule> = vec![];
        let mut rule = String::new();
        for cap in captures {
            let label = parser.label_for(cap.id());
            println!("Label: {label:?}");
            match label {
                "text" => {
                    let range = cap.range();
                    let text = &pattern[range.start as usize..range.end as usize];
                    rule.push('"');
                    rules.push(Rule::ByteSequence(text.as_bytes().to_vec()))
                }
                _ => {
                    rule.push(' ');
                    rule.push_str(label);
                }
            }
        }

        let info = RuleInfo {
            top: true,
            annotations: vec![],
            name: "glob".into(),
            rule: Rule::Sequence(rules),
        };
        let rules = Rules::new(Box::new([info]));

        let pparse = Parser::from_rules(rules).unwrap();
        Glob { parser: pparse }
    }

    pub fn is_match<B: AsRef<[u8]>>(&self, bytes: &B) -> bool {
        let bytes = bytes.as_ref();
        match self.parser.parse(bytes) {
            Ok(_) => true,
            Err(e) => {
                println!("Error: {e}");
                false
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn glob() {
        let glob = Glob::new(".*");
        println!("{}", glob.is_match(b".shit"));
        println!("{}", glob.is_match(b"path/to/glob.rs"));
        println!("{}", glob.is_match(b"text/lorem.txt"));
    }
}
