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

struct Paren {
    natom: usize,
    nalt: usize,
    nparen: usize,
}

pub(crate) fn regex_to_postfix(re: &str) -> Postfix {
    let mut buf = Vec::new();
    let mut parens = Vec::new();
    let mut nparen = 0;
    let mut natom = 0;
    let mut nalt = 0;

    for ch in re.chars() {
        match ch {
            '(' => {
                if natom > 1 {
                    natom -= 1;
                    buf.push(PF::Seq);
                }
                buf.push(PF::Save(nparen * 2));
                let paren = Paren {
                    natom,
                    nalt,
                    nparen,
                };
                nparen += 1;
                natom = 0;
                nalt = 0;
                parens.push(paren);
            }
            '|' => {
                debug_assert!(natom != 0);
                natom -= 1;
                while natom > 0 {
                    buf.push(PF::Seq);
                    natom -= 1;
                }
                nalt += 1;
            }
            ')' => {
                debug_assert!(!parens.is_empty());
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

                let last = parens.pop().expect("no parens found");

                buf.push(PF::Save((last.nparen * 2) + 1));
                natom = last.natom;
                nalt = last.nalt;
                natom += 1;
            }
            '*' => {
                debug_assert!(natom != 0);
                buf.push(PF::Star(false));
            }
            '+' => {
                debug_assert!(natom != 0);
                buf.push(PF::Plus(false));
            }
            '?' => {
                debug_assert!(natom != 0);

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
                if natom > 1 {
                    natom -= 1;
                    buf.push(PF::Seq);
                }
                buf.push(PF::Any);
                natom += 1;
            }
            _ => {
                if natom > 1 {
                    natom -= 1;
                    buf.push(PF::Seq);
                }
                buf.push(PF::Char(ch));
                natom += 1;
            }
        }
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn simple() {
        let postfix = regex_to_postfix("cat|(dog)");
        println!("Pfix {postfix:?}");
    }
}
