use std::str::Chars;

use super::ast::Ast;

pub(crate) struct Parser<'a> {
    chars: Chars<'a>,
    next: Option<char>,
}

impl<'a> Parser<'a> {
    /// Parse a regex pattern to an AST
    pub fn parse(regex: &str) -> Ast {
        let mut parser = Parser::new(regex);
        parser.expr()
    }

    fn new(regex: &'a str) -> Parser<'a> {
        let mut chars = regex.chars();
        let next = chars.next();

        Parser { chars, next }
    }

    // Eat away one char from the regex
    fn eat(&mut self, ch: char) {
        if self.next == Some(ch) {
            self.skip();
        } else {
            panic!("Tried to eat char {ch} and next was {:?}", self.next);
        }
    }

    fn skip(&mut self) {
        self.next = self.chars.next();
    }

    fn next(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.eat(ch);
        Some(ch)
    }

    // peek the next char in the regex
    fn peek(&self) -> Option<char> {
        self.next
    }

    fn expr(&mut self) -> Ast {
        self.alt()
    }

    fn alt(&mut self) -> Ast {
        let term = self.seq();
        let mut alt = vec![term];

        while self.peek().map(|ch| matches!(ch, '|')).unwrap_or(false) {
            self.skip();
            let ast = self.seq();
            alt.push(ast);
        }

        if alt.len() == 1 {
            alt.remove(0)
        } else {
            Ast::Alt(alt)
        }
    }

    fn seq(&mut self) -> Ast {
        let mut seq = vec![];

        while self
            .peek()
            .map(|ch| !matches!(ch, '|' | ')'))
            .unwrap_or(false)
        {
            let next = self.rep();
            seq.push(next);
        }

        if seq.len() == 1 {
            seq.remove(0)
        } else {
            Ast::Seq(seq)
        }
    }

    fn rep(&mut self) -> Ast {
        let base = self.base();

        match self.peek() {
            Some('*') => {
                self.skip();
                Ast::Star(Box::new(base), self.next_lazy())
            }
            Some('?') => {
                self.skip();
                Ast::Question(Box::new(base), self.next_lazy())
            }
            Some('+') => {
                self.skip();
                Ast::Plus(Box::new(base), self.next_lazy())
            }
            // Some('{') => {} {2}
            _ => base,
        }
    }

    fn next_lazy(&mut self) -> bool {
        if let Some('?') = self.peek() {
            self.skip();
            true
        } else {
            false
        }
    }

    fn base(&mut self) -> Ast {
        match self.peek() {
            Some('.') => {
                self.skip();
                Ast::Any
            }
            Some('(') => {
                self.eat('(');
                let ast = self.expr();
                self.eat(')');
                Ast::Group(ast.into())
            }
            Some('\\') => {
                self.skip();
                let ch = self.next().expect("escaped char");
                Ast::Char(ch)
            }
            // Some('[') => {} [a-z]
            Some(ch) => {
                self.skip();
                Ast::Char(ch)
            }
            None => unreachable!(),
        }
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
    fn complex() {
        let ast = Parser::parse("(a??b+c*)|b|d");
        println!("AST {ast:?}");
    }
}
