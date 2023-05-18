// Store regexs saved positions, This creates a slot vector for all
// possible instructions in the program, even numbers are start points and odd
// numbers are end points.
#[derive(Debug)]
pub(crate) struct Slots {
    slots: Vec<Box<[Option<usize>]>>,
}

impl Slots {
    pub fn new(slot_count: usize, program_len: usize) -> Slots {
        let slots = vec![vec![None; slot_count].into(); program_len];
        Slots { slots }
    }

    pub fn copy(&mut self, from: usize, to: usize) {
        if from == to {
            return;
        }

        if from < to {
            let (head, tail) = self.slots.split_at_mut(from + 1);
            let to = &mut tail[to - from - 1];
            let from = &head[from];

            for i in 0..to.len() {
                to[i] = from[i];
            }
        } else {
            let (head, tail) = self.slots.split_at_mut(to + 1);
            let from = &tail[from - to - 1];
            let to = &mut head[to];

            for i in 0..to.len() {
                to[i] = from[i];
            }
        }
    }

    pub fn get(&mut self, slot: usize) -> &mut [Option<usize>] {
        self.slots[slot].as_mut()
    }

    pub fn get_as_pairs(&mut self, pos: usize) -> Vec<(usize, usize)> {
        let slots = &self.slots[pos];
        let mut pairs = Vec::with_capacity(slots.len() / 2);

        for i in (0..slots.len()).step_by(2) {
            let start = slots[i];
            let end = slots[i + 1];

            match (start, end) {
                (Some(s), Some(e)) => {
                    pairs.push((s, e));
                }
                _ => {}
            }
        }

        pairs
    }
}
