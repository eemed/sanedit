use std::{
    iter::Peekable,
    str::{CharIndices, Chars},
};

#[derive(Debug)]
pub struct ParseError {
    pos: usize,
    kind: ParseErrorKind,
}

#[derive(Debug)]
pub enum ParseErrorKind {
    Number,
}

pub(crate) type Postfix = Vec<PF>;

#[derive(Debug, Clone)]
pub(crate) enum PF {
    Char(char),
    Seq,
    Or,
    Star(bool),
    Plus(bool),
    Question(bool),
    Save(usize),
    Any,
    Range(u8, u8),
    Repeat(u32),
    // Range(char, char),
}

pub(crate) fn literal_to_postfix(string: &str) -> Postfix {
    let mut buf = Vec::new();
    for (i, ch) in string.chars().enumerate() {
        buf.push(PF::Char(ch));

        if i % 2 == 0 && i != 0 {
            buf.push(PF::Seq);
        }
    }

    buf
}

#[derive(Debug)]
pub enum Op {
    Paren(usize),
    Or,
    Seq,
}

impl TryFrom<Op> for PF {
    type Error = String;

    fn try_from(value: Op) -> Result<Self, Self::Error> {
        match value {
            Op::Paren(_) => todo!(),
            Op::Or => Ok(PF::Or),
            Op::Seq => Ok(PF::Seq),
        }
    }
}

// https://en.wikipedia.org/wiki/Shunting_yard_algorithm
//
// shunting yard algorithm used as a base but extended to handle postfix
// operators and create the missing sequence infix operators.
pub(crate) fn shunting_yard(re: &str) -> Result<Postfix, ParseError> {
    use PF::*;
    let mut operators = Vec::new();
    let mut output = Vec::new();
    let mut nparen = 0;
    let mut lastch = None;
    let mut chars = re.char_indices().peekable();

    while let Some((pos, mut ch)) = chars.next() {
        // create infix sequence operators between atoms
        let lastatom = lastch.map(|ch| !matches!(ch, '|' | '(')).unwrap_or(false);
        let atom = !matches!(ch, '|' | ')' | '*' | '?' | '+');
        if lastatom && atom {
            operators.push(Op::Seq);
        }

        match ch {
            '{' => {
                let mut num = String::new();
                while let Some((_, ch)) = chars.next() {
                    if ch == '}' {
                        break;
                    }

                    num.push(ch);
                }

                ch = '}';
                let num = num.parse::<u32>().map_err(|_e| ParseError {
                    pos,
                    kind: ParseErrorKind::Number,
                })?;
                output.push(Repeat(num));
            }
            '\\' => {
                if let Some((_, next)) = chars.next() {
                    ch = next;
                    output.push(Char(next));
                }
            }
            '(' => {
                operators.push(Op::Paren(nparen));
                output.push(Save(nparen * 2));
                nparen += 1;
            }
            ')' => {
                while let Some(op) = operators.pop() {
                    if let Op::Paren(n) = op {
                        output.push(Save(n * 2 + 1));
                        break;
                    }
                    output.push(op.try_into().unwrap());
                }
            }
            '[' => {
                let mut pf = shunting_yard_list(&mut chars)?;
                output.append(&mut pf);
            }
            '|' => operators.push(Op::Or),
            '.' => output.push(Any),
            '*' => {
                let lazy = chars.next_if(|(_, ch)| matches!(ch, '?')).is_some();
                output.push(Star(lazy));
            }
            '+' => {
                let lazy = chars.next_if(|(_, ch)| matches!(ch, '?')).is_some();
                output.push(Plus(lazy));
            }
            '?' => {
                let lazy = chars.next_if(|(_, ch)| matches!(ch, '?')).is_some();
                output.push(Question(lazy));
            }
            _ => output.push(Char(ch)),
        }

        lastch = Some(ch);
    }

    while let Some(op) = operators.pop() {
        output.push(op.try_into().unwrap());
    }

    Ok(output)
}

fn shunting_yard_list(chars: &mut Peekable<CharIndices>) -> Result<Postfix, ParseError> {
    use PF::*;
    let mut operators = Vec::new();
    let mut output = Vec::new();

    while let Some((pos, ch)) = chars.next() {
        if ch == ']' {
            break;
        }

        if !output.is_empty() {
            operators.push(Op::Or);
        }

        let has_dash = chars.next_if(|(_, ch)| matches!(ch, '-')).is_some();
        if has_dash {
            if let Some((_, end)) = chars.next_if(|(_, ch)| !matches!(ch, ']')) {
                let start = ch as u8;
                let end = end as u8;
                output.push(Range(start, end));
            } else {
                output.push(Char(ch));
                output.push(Char('-'));
            }
        } else {
            output.push(Char(ch));
        }
    }

    while let Some(op) = operators.pop() {
        output.push(op.try_into().unwrap());
    }

    Ok(output)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn simple() {
        // let regex = "a(b|c)*d[a-zE]f";
        // let regex = "a(b|c)*d";
        // let regex = "a(b|c)*d??abc";
        // let regex = "a\\({2}[a-z-]d";
        // let regex = "a+z|a";
        let regex = "(a+z|a)+(z+a)*";
        println!("----- {regex} --------");
        let postfix = shunting_yard(regex);
        println!("{postfix:?}");

        // let postfix = regex2postfix(regex);
        // println!("NPF: {postfix:?}");
    }
}
