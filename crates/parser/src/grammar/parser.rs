use crate::input::Input;
use anyhow::anyhow;
use anyhow::bail;
use anyhow::ensure;
use anyhow::Result;

#[derive(Debug)]
struct Rule {
    name: String,
    expr: Expr,
}

#[derive(Debug)]
enum Expr {
    Literal(String),
}

struct Parser<I: Input> {
    input: I,
}

impl<I: Input> Parser<I> {
    fn parse(input: I) -> Result<Vec<Rule>> {
        let mut parser = Parser { input };
        let mut rules = vec![];
        while parser.input.peek().is_some() {
            let rule = parser.rule()?;
            rules.push(rule);
            parser.consume_whitespace()?;
        }

        Ok(rules)
    }

    fn consume(&mut self, s: &str) -> Result<()> {
        self.consume_whitespace()?;

        let chars = s.chars();

        for ch in chars {
            self.input.consume(ch)?;
        }

        Ok(())
    }

    fn consume_whitespace(&mut self) -> Result<()> {
        while let Some(ch) = self.input.peek() {
            if ch.is_whitespace() {
                self.input.consume(ch)?;
            } else {
                break;
            }
        }
        Ok(())
    }

    fn rule(&mut self) -> Result<Rule> {
        let ident = self.identifier()?;
        self.consume("<-")?;
        let expr = self.expr()?;

        Ok(Rule { name: ident, expr })
    }

    fn identifier(&mut self) -> Result<String> {
        self.consume_whitespace()?;

        let mut ident = String::new();
        while let Some(ch) = self.input.peek() {
            if ch.is_alphabetic() {
                ident.push(ch);
                self.input.consume(ch)?;
            } else {
                break;
            }
        }

        ensure!(
            !ident.is_empty(),
            "Tried to parse empty identifier at {}",
            self.input.pos()
        );

        Ok(ident)
    }

    fn expr(&mut self) -> Result<Expr> {
        self.consume_whitespace()?;

        if let Some(ch) = self.input.peek() {
            match ch {
                '"' => self.literal(),
                // '(' => {}
                // '*' => {}
                // '?' => {}
                // '+' => {}
                _ => bail!("Failed to parse expr at {}", self.input.pos()),
            }
        } else {
            bail!(
                "Tried to parse expr but input ended at {}",
                self.input.pos()
            )
        }
    }

    fn literal(&mut self) -> Result<Expr> {
        self.consume_whitespace()?;
        self.consume("\"")?;
        let lit = self.identifier()?;
        self.consume("\"")?;
        Ok(Expr::Literal(lit))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::input::StringInput;

    #[test]
    fn parse() {
        let input = StringInput::new("rule <- \"foobar\"");
        let res = Parser::parse(input);
        println!("res: {res:?}");
    }
}
