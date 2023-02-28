use std::str::Chars;

use crate::Ast;

pub fn parse(regex: &str) -> Ast {
    let parser = Parser::new(regex);
    parser.parse()
}

struct Parser<'a> {
    chars: Chars<'a>,
    next: Option<char>,
    parsed: Vec<Ast>,
}

impl<'a> Parser<'a> {
    pub fn new(regex: &'a str) -> Parser<'a> {
        let mut chars = regex.chars();
        let next = chars.next();

        Parser {
            chars,
            next,
            parsed: vec![],
        }
    }

    pub fn parse(mut self) -> Ast {
        loop {
            match self.peek() {
                Some('*') => {
                    self.eat();
                    let ast = self.parsed.pop().expect("* no ast to pop");
                    self.parsed.push(Ast::Star(Box::new(ast)));
                }
                Some('+') => {
                    self.eat();
                    let ast = self.parsed.pop().expect("+ no ast to pop");
                    self.parsed.push(Ast::Plus(Box::new(ast)));
                }
                Some('?') => {
                    self.eat();
                    let ast = self.parsed.pop().expect("? no ast to pop");
                    self.parsed.push(Ast::Question(Box::new(ast)));
                }
                Some(ch) => {
                    self.eat();
                    self.parsed.push(Ast::Char(ch));
                }
                None => break,
            }
        }

        Ast::Concat(self.parsed)
    }

    // Eat away one char from the regex
    pub fn eat(&mut self) {
        self.next = self.chars.next();
    }

    // peek the next char in the regex
    pub fn peek(&self) -> Option<char> {
        self.next
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[macro_export]
    macro_rules! get {
        ($value:expr, $pattern:pat => $extracted_value:expr) => {
            match $value {
                $pattern => $extracted_value.clone(),
                _ => panic!("Pattern doesn't match!"),
            }
        };
    }

    #[test]
    fn parse_chars() {
        let ast = parse("abc");
        assert!(matches!(ast, Ast::Concat(..)));
        let chars = get!(ast, Ast::Concat(v) => v);
        match &chars[..] {
            [a, b, c] => {
                assert!(matches!(a, Ast::Char('a')));
                assert!(matches!(b, Ast::Char('b')));
                assert!(matches!(c, Ast::Char('c')));
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn parse_plus() {
        let ast = parse("a+b");
        assert!(matches!(ast, Ast::Concat(..)));
        let chars = get!(ast, Ast::Concat(v) => v);
        match &chars[..] {
            [p, b] => {
                let a = get!(p, Ast::Plus(p) => p);
                assert!(matches!(*a, Ast::Char('a')));
                assert!(matches!(b, Ast::Char('b')));
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn parse_question() {
        let ast = parse("a?b");
        assert!(matches!(ast, Ast::Concat(..)));
        let chars = get!(ast, Ast::Concat(v) => v);
        match &chars[..] {
            [p, b] => {
                let a = get!(p, Ast::Question(p) => p);
                assert!(matches!(*a, Ast::Char('a')));
                assert!(matches!(b, Ast::Char('b')));
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn parse_star() {
        let ast = parse("a*b");
        assert!(matches!(ast, Ast::Concat(..)));
        let chars = get!(ast, Ast::Concat(v) => v);
        match &chars[..] {
            [s, b] => {
                let a = get!(s, Ast::Star(p) => p);
                assert!(matches!(*a, Ast::Char('a')));
                assert!(matches!(b, Ast::Char('b')));
            }
            _ => unreachable!(),
        }
    }
}
