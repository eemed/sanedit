pub(crate) type Postfix = Vec<PF>;

#[derive(Debug)]
pub(crate) enum PF {
    Char(char),
    Seq,
    Or,
    Star(bool),
    Plus(bool),
    Question(bool),
    Save(usize),
    Any,
}

impl PF {
    fn is_atom(&self) -> bool {
        matches!(self, PF::Char(_) | PF::Any)
    }
}

struct Saved {
    natom: usize,
    nalt: usize,
    nparen: usize,
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
pub(crate) fn shunting_yard(re: &str) -> Postfix {
    use PF::*;
    let mut operators = Vec::new();
    let mut output = Vec::new();
    let mut nparen = 0;
    let mut lastch = None;
    let mut chars = re.chars().peekable();

    while let Some(ch) = chars.next() {
        // create infix sequence operators between atoms
        let lastatom = lastch.map(|ch| !matches!(ch, '|' | '(')).unwrap_or(false);
        let atom = !matches!(ch, '|' | ')' | '*' | '?' | '+');
        if lastatom && atom {
            operators.push(Op::Seq);
        }

        match ch {
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
            '|' => operators.push(Op::Or),
            '.' => output.push(Any),
            '*' => {
                let lazy = chars.next_if(|ch| matches!(ch, '?')).is_some();
                output.push(Star(lazy));
            }
            '+' => {
                let lazy = chars.next_if(|ch| matches!(ch, '?')).is_some();
                output.push(Plus(lazy));
            }
            '?' => {
                let lazy = chars.next_if(|ch| matches!(ch, '?')).is_some();
                output.push(Question(lazy));
            }
            _ => output.push(Char(ch)),
        }

        lastch = Some(ch);
    }

    while let Some(op) = operators.pop() {
        output.push(op.try_into().unwrap());
    }

    output
}

pub(crate) fn regex2postfix(re: &str) -> Postfix {
    let mut buf = Vec::new();
    let mut parens = Vec::new();
    let mut group = None;
    let mut nparen = 0;
    let mut natom = 0;
    let mut nalt = 0;

    for ch in re.chars() {
        if group.is_none() {
            regex_to_postfix(
                &mut buf,
                &mut parens,
                &mut group,
                &mut nparen,
                &mut natom,
                &mut nalt,
                ch,
            );
        }

        if group.is_some() {}
    }

    debug_assert!(parens.is_empty());
    debug_assert!(natom != 0);

    natom -= 1;
    while natom > 0 {
        buf.push(PF::Seq);
        natom -= 1;
    }
    while nalt > 0 {
        buf.push(PF::Or);
        nalt -= 1;
    }

    buf
}

fn regex_to_postfix_group(
    buf: &mut Vec<PF>,
    parens: &mut Vec<Saved>,
    group: &mut Option<Saved>,
    nparen: &mut usize,
    natom: &mut usize,
    nalt: &mut usize,
    ch: char,
) {
}

fn regex_to_postfix(
    buf: &mut Vec<PF>,
    parens: &mut Vec<Saved>,
    group: &mut Option<Saved>,
    nparen: &mut usize,
    natom: &mut usize,
    nalt: &mut usize,
    ch: char,
) {
    // let mut buf = Vec::new();
    // let mut parens = Vec::new();
    // let mut groups = Vec::new();
    // let mut nparen = 0;
    // let mut natom = 0;
    // let mut nalt = 0;

    // for ch in re.chars() {
    match ch {
        '(' => {
            if *natom > 1 {
                *natom -= 1;
                buf.push(PF::Seq);
            }
            buf.push(PF::Save(*nparen * 2));
            let paren = Saved {
                natom: *natom,
                nalt: *nalt,
                nparen: *nparen,
            };
            *nparen += 1;
            *natom = 0;
            *nalt = 0;
            parens.push(paren);
        }
        '|' => {
            debug_assert!(*natom != 0);
            *natom -= 1;
            while *natom > 0 {
                buf.push(PF::Seq);
                *natom -= 1;
            }
            *nalt += 1;
        }
        ')' => {
            debug_assert!(!parens.is_empty());
            debug_assert!(*natom != 0);
            *natom -= 1;
            while *natom > 0 {
                buf.push(PF::Seq);
                *natom -= 1;
            }
            while *nalt > 0 {
                buf.push(PF::Or);
                *nalt -= 1;
            }

            let last = parens.pop().expect("no parens found");

            buf.push(PF::Save((last.nparen * 2) + 1));
            *natom = last.natom;
            *nalt = last.nalt;
            *natom += 1;
        }
        '*' => {
            debug_assert!(*natom != 0);
            buf.push(PF::Star(false));
        }
        '+' => {
            debug_assert!(*natom != 0);
            buf.push(PF::Plus(false));
        }
        '?' => {
            debug_assert!(*natom != 0);

            if let Some(last) = buf.pop() {
                match last {
                    PF::Star(false) => buf.push(PF::Star(true)),
                    PF::Plus(false) => buf.push(PF::Plus(true)),
                    PF::Question(false) => buf.push(PF::Question(true)),
                    _ => {
                        buf.push(last);
                        buf.push(PF::Question(false));
                    }
                }
            } else {
                buf.push(PF::Question(false));
            }
        }
        '.' => {
            if *natom > 1 {
                *natom -= 1;
                buf.push(PF::Seq);
            }
            buf.push(PF::Any);
            *natom += 1;
        }
        '[' => {
            let saved = Saved {
                natom: *natom,
                nalt: *nalt,
                nparen: *nparen,
            };
            *group = Some(saved);
        }
        _ => {
            if *natom > 1 {
                *natom -= 1;
                buf.push(PF::Seq);
            }
            buf.push(PF::Char(ch));
            *natom += 1;
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn simple() {
        // let regex = "a(b|c)*d[a-zE]f";
        // let regex = "a(b|c)*d";
        let regex = "a(b|c)*d??abc";
        println!("----- {regex} --------");
        let postfix = shunting_yard(regex);
        println!("SYA: {postfix:?}");

        let postfix = regex2postfix(regex);
        println!("NPF: {postfix:?}");
    }
}
