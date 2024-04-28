use std::io;

use crate::grammar::{self, Rule};

enum Operation {
    Jump,
    Char,
    Commit,
    Choice,
    Return,
    Fail,
    End,
    EndFail,
}

struct Parser {}

impl Parser {
    pub fn new<R: io::Read>(read: R) -> Parser {
        let rules = grammar::parse_rules(read).unwrap();
        compile(&rules);
        todo!()
    }
}

fn compile(rules: &[Rule]) {
    let mut program = vec![];
}

fn compile_rec(rule: &Rule, rules: &[Rule]) {
    use grammar::RuleDefinition::*;

    match rule.def {
        Optional(_) => todo!(),
        ZeroOrMore(_) => todo!(),
        OneOrMore(_) => todo!(),
        Choice(_) => todo!(),
        Sequence(_) => todo!(),
        FollowedBy(_) => todo!(),
        NotFollowedBy(_) => todo!(),
        CharSequence(_) => todo!(),
        CharRange(_, _) => todo!(),
        Ref(_) => todo!(),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn compiler() {
        let peg = include_str!("../pegs/json.peg");
        let rules = grammar::parse_rules(std::io::Cursor::new(peg)).unwrap();
        compile(&rules);

        // let parser = PikaParser::from_str(peg).unwrap();
        // let input = " {\"account\":\"bon\",\n\"age\":3.2, \r\n\"children\" : [  1, 2,3], \"allow-children\": true } ";
        // let ast = parser.parse(input).unwrap();
        // ast.print(input);
    }
}
