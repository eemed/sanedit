#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Either<L, R> {
    Left(L),
    Right(R),
}

impl<L, R> Either<L, R> {
    pub fn take_left(self) -> Option<L> {
        match self {
            Either::Left(l) => Some(l),
            Either::Right(_) => panic!("left called when either was right"),
        }
    }

    pub fn take_right(self) -> Option<R> {
        match self {
            Either::Left(_) => panic!("left called when either was right"),
            Either::Right(r) => Some(r),
        }
    }

    pub fn left(&self) -> Option<&L> {
        match self {
            Either::Left(l) => Some(l),
            Either::Right(_) => panic!("left called when either was right"),
        }
    }

    pub fn right(&self) -> Option<&R> {
        match self {
            Either::Left(_) => panic!("left called when either was right"),
            Either::Right(r) => Some(r),
        }
    }

    pub fn is_right(&self) -> bool {
        matches!(self, Either::Right(_))
    }

    pub fn is_left(&self) -> bool {
        !self.is_right()
    }
}
