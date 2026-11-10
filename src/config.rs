use std::marker::PhantomData;
use std::{collections::HashMap, time::Duration};

use crate::buffer::{Buffer, StrIndex, StringInterner};
use rand::{Rng, RngExt};
use serde::Deserialize;

#[derive(Clone, Copy)]
#[repr(usize)]
pub(crate) enum SessionKind {
    Heavy = 0,
    Light = 1,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawConfig<'a> {
    specs: HashMap<&'a str, RawSpec<'a>>,
    url: &'a str,
}
type RawSpec<'a> = HashMap<&'a str, Option<(Option<u8>, Option<u8>)>>;

type Brand<'id> = PhantomData<fn(&'id ()) -> &'id ()>;

#[derive(Eq, PartialEq, Clone, Copy)]
pub(crate) struct ExerciseIndex<'id> {
    idx: u16,
    _brand: Brand<'id>,
}

#[derive(Debug)]
pub(crate) struct Config<'id> {
    pub(crate) duration: Duration,
    url: StrIndex<'id>,
    buffer: Buffer<'id>,

    groups: HashMap<StrIndex<'id>, (u16, u16)>,
    names: Box<[StrIndex<'id>]>,
    weights: Box<[u8]>,
}

impl<'id> Config<'id> {
    pub(crate) fn get_url(&self) -> &str {
        self.buffer.get(self.url)
    }
    pub(crate) fn get_group(&self, idx: StrIndex<'id>) -> &str {
        self.buffer.get(idx)
    }
    pub(crate) fn get_name(&self, idx: ExerciseIndex<'id>) -> &str {
        self.buffer.get(self.names[usize::from(idx.idx)])
    }
    pub(crate) fn get_weight(&self, idx: ExerciseIndex<'id>) -> u8 {
        self.weights[usize::from(idx.idx)]
    }

    pub(crate) fn get_exercise(
        &self,
        rng: &mut impl Rng,
        group: StrIndex<'id>,
    ) -> Option<ExerciseIndex<'id>> {
        let (start, stop) = self.groups.get(&group)?;
        let idx = rng.random_range(*start..*stop);
        Some(ExerciseIndex { idx, _brand: PhantomData })
    }

    pub(crate) fn from_raw<'a>(
        mut interner: StringInterner<'a, 'id>,
        raw: RawConfig<'a>,
        duration: Duration,
        session: SessionKind,
    ) -> eyre::Result<Config<'id>> {
        let url = interner.insert(raw.url);
        let capacity = raw.specs.len();
        let mut names = Vec::with_capacity(capacity);
        let mut weights = Vec::with_capacity(capacity);
        let mut groups = HashMap::with_capacity(capacity);

        let mut offset = names.len();
        for (group, spec) in &raw.specs {
            for (name, var) in spec {
                weights.push(match (session, var) {
                    (SessionKind::Light, Some((Some(w), _))) => *w,
                    (SessionKind::Heavy, Some((_, Some(w)))) => *w,
                    (_, None) => 0,
                    _ => continue,
                });
                let idx = interner.insert(name);
                names.push(idx);
            }
            assert!(names.len() <= u16::MAX.into());
            groups.insert(
                interner.insert(group),
                (offset as u16, names.len() as u16),
            );
            offset = names.len();
        }

        assert!(names.len() < u16::MAX.into());
        Ok(Config {
            buffer: interner.into_buffer(),
            url,
            duration,
            names: names.into(),
            groups,
            weights: weights.into(),
        })
    }
}
