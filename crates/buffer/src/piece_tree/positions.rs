#[derive(Debug, Clone)]
pub enum SortedPositions<'a> {
    Ref(&'a [usize]),
    Owned(Vec<usize>),
}

impl<'a> SortedPositions<'a> {
    pub fn new(positions: &'a [usize]) -> SortedPositions<'a> {
        if is_sorted(positions) {
            Self::Ref(positions)
        } else {
            let mut positions = positions.to_vec();
            positions.sort();
            Self::Owned(positions)
        }
    }

    pub fn iter(&self) -> std::slice::Iter<usize> {
        match self {
            SortedPositions::Ref(poss) => poss.iter(),
            SortedPositions::Owned(poss) => poss.iter(),
        }
    }
}

impl<'a> From<Vec<usize>> for SortedPositions<'a> {
    fn from(mut arr: Vec<usize>) -> Self {
        arr.sort();
        Self::Owned(arr)
    }
}

impl<'a> From<usize> for SortedPositions<'a> {
    fn from(value: usize) -> Self {
        Self::Owned(vec![value])
    }
}

fn is_sorted(arr: &[usize]) -> bool {
    let mut min = 0;

    for item in arr {
        if min > *item {
            return false;
        }

        min = *item;
    }

    true
}
