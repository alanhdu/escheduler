use chacha20::ChaCha12Rng;
use rand::{Rng, SeedableRng};
use rand_seeder::Seeder;

pub(crate) struct Session {
    session: u32,
    rng: ChaCha12Rng,
}

impl Session {
    pub(crate) fn from_session_number(session: u32) -> Self {
        Self { session, rng: stream_rng("exercises", 0, u64::from(session)) }
    }

    pub(crate) fn rng(&mut self) -> ChaCha12Rng {
        ChaCha12Rng::from_rng(&mut self.rng)
    }

    /// Select each input slot once per aligned block of N sessions, starting
    /// at session zero. Duplicate values act as integer weights.
    ///
    /// Give each logical choice a unique, stable stream name. Its result is
    /// independent of other calls and their order, including ordinary RNG use.
    /// Repeating a stream within one session repeats the same choice. Balance
    /// requires a fixed input order and length across sessions.
    pub(crate) fn balanced_select<T, const N: usize>(
        &self,
        stream: &str,
        mut xs: [T; N],
    ) -> T {
        assert!(N > 0, "balanced_select requires at least one option");
        self.balanced_shuffle(stream, &mut xs);
        xs.into_iter().next().unwrap()
    }

    /// Balance every output position: in each aligned block of n sessions,
    /// every input slot appears exactly once at each output position.
    ///
    /// Each block has a fresh permutation and a fresh permutation of its
    /// rotations (a randomized Latin square). Rows within a block must be
    /// dependent to preserve exact balance; different streams and blocks use
    /// separately seeded randomness. Repeated permutations are allowed.
    ///
    /// Uses the same stream and fixed-input contract as balanced_select.
    /// Empty and singleton slices are unchanged.
    pub(crate) fn balanced_shuffle<T>(&self, stream: &str, xs: &mut [T]) {
        if xs.len() <= 1 {
            return;
        }
        let n = xs.len() as u64;
        let session = u64::from(self.session);
        let mut rng = stream_rng(stream, n, session / n);
        shuffle(xs, &mut rng);
        let mut rotations: Vec<_> = (0..xs.len()).collect();
        shuffle(&mut rotations, &mut rng);
        xs.rotate_left(rotations[(session % n) as usize]);
    }
}

fn stream_rng(stream: &str, len: u64, block: u64) -> ChaCha12Rng {
    Seeder::from((20260326_u64, stream, len, block)).into_rng()
}

// Fisher-Yates with unbiased rejection sampling. Keeping the sampling algorithm
// here and naming ChaCha12 explicitly makes balanced output reproducible across
// platforms and independent of changes to rand's StdRng or SliceRandom.
fn shuffle<T>(xs: &mut [T], rng: &mut impl Rng) {
    for i in (1..xs.len()).rev() {
        let bound = (i + 1) as u64;
        let threshold = bound.wrapping_neg() % bound;
        let index = loop {
            let value = rng.next_u64();
            if value >= threshold {
                break (value % bound) as usize;
            }
        };
        xs.swap(i, index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn balanced_blocks() {
        fn check<const N: usize>(options: [u8; N]) {
            let n = N as u32;
            let mut expected = options;
            expected.sort_unstable();
            for block in [0, 1, 255, 256, u32::MAX / n - 1] {
                let mut columns = vec![Vec::new(); N];
                let mut selections = Vec::new();
                for offset in 0..n {
                    let session =
                        Session::from_session_number(block * n + offset);
                    let mut row = options;
                    session.balanced_shuffle("choice", &mut row);
                    let selected = session.balanced_select("choice", options);
                    assert_eq!(selected, row[0]);
                    selections.push(selected);
                    for (column, &value) in columns.iter_mut().zip(&row) {
                        column.push(value);
                    }
                    row.sort_unstable();
                    assert_eq!(row, expected, "block={block}, offset={offset}");
                }
                for mut column in columns.into_iter().chain([selections]) {
                    column.sort_unstable();
                    assert_eq!(column, expected, "block={block}");
                }
            }
        }

        check([0]);
        check([0, 1]);
        check([0, 1, 2]);
        check([0, 1, 1]); // Duplicate values act as weights.
        check([0, 1, 2, 3, 4, 5, 6]);
        let mut empty: [u8; 0] = [];
        Session::from_session_number(0).balanced_shuffle("empty", &mut empty);
        assert!(empty.is_empty());
    }

    #[test]
    fn deterministic_streams() {
        let ordering = |session: &Session, stream| {
            let mut xs = [0, 1, 2, 3, 4, 5, 6, 7];
            session.balanced_shuffle(stream, &mut xs);
            xs
        };
        for (stream, number) in
            [("lower", 0), ("core", 1), ("core", 768), ("aux", u32::MAX)]
        {
            let mut session = Session::from_session_number(number);
            let expected = ordering(&session, stream);
            let first_draw = session.rng().next_u64();
            session.balanced_select("unrelated", [0, 1, 2]);
            let _ = session.rng().next_u64();
            assert_eq!(ordering(&session, stream), expected);
            let mut replay = Session::from_session_number(number);
            assert_eq!(ordering(&replay, stream), expected);
            assert_eq!(replay.rng().next_u64(), first_draw);
            assert_ne!(session.rng().next_u64(), first_draw);
            assert_ne!(
                Session::from_session_number(number.wrapping_add(1))
                    .rng()
                    .next_u64(),
                first_draw,
            );
        }

        // Compare sequences, allowing individual orderings to repeat by chance.
        let sequence = |stream, start| {
            (start..start + 24)
                .map(|number| {
                    ordering(&Session::from_session_number(number), stream)
                })
                .collect::<Vec<_>>()
        };
        for (stream, start) in [("other", 0), ("core", 8), ("core", 8 * 256)] {
            assert_ne!(sequence("core", 0), sequence(stream, start));
        }
    }
}
