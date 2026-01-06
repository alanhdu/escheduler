use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::Read,
    path::Path,
    time::Duration,
};

use eyre::Context;
use rand::{Rng, seq::SliceRandom};
use serde::Deserialize;

use crate::buffer::{Buffer, BufferBuilder, Index};

#[derive(Deserialize, Debug, Clone, Copy, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Target {
    Core,
    Lower,
    Upper,
}

pub(crate) struct TargetOrder([Target; 3]);

impl TargetOrder {
    pub(crate) fn new(rng: &mut impl Rng) -> Self {
        let mut order = [Target::Core, Target::Lower, Target::Upper];
        order.shuffle(rng);
        TargetOrder(order)
    }

    pub(crate) fn first(&self) -> Target {
        self.0[0]
    }

    pub(crate) fn next(&self, current: Target) -> Target {
        if current == self.0[0] {
            self.0[1]
        } else if current == self.0[1] {
            self.0[2]
        } else if current == self.0[2] {
            self.0[0]
        } else {
            unreachable!()
        }
    }
}

impl std::fmt::Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let target = match self {
            Target::Core => "core",
            Target::Lower => "lower",
            Target::Upper => "upper",
        };
        write!(f, "{}", target)
    }
}

// TODO: in theory, these should be Cow<'a, str> instead
// to deal with escapes, but let's just assume we don't have any
// for now.
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct Spec<'a> {
    #[serde(borrow)]
    #[serde(default)]
    variations: Option<Vec<&'a str>>,

    #[serde(default)]
    needs_weight: bool,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct Specs<'a> {
    #[serde(borrow)]
    upper: HashMap<&'a str, Spec<'a>>,
    lower: HashMap<&'a str, Spec<'a>>,
    core: HashMap<&'a str, Spec<'a>>,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct ConfigBuilder<'a> {
    specs: Specs<'a>,
    url: &'a str,
}

#[derive(Debug)]
pub(crate) struct Config {
    pub(crate) duration: Duration,
    pub(crate) weight: u8,

    buffer: Buffer,
    indices: [u16; 4],

    names: Box<[Index]>,
    groups: Box<[Index]>,
    needs_weights: HashSet<u16>,

    url: Index,
}

pub(crate) struct Exercise<'a> {
    pub(crate) target: Target,
    pub(crate) name: &'a str,
    pub(crate) group: &'a str,
    pub(crate) needs_weight: bool,
}

impl Config {
    pub(crate) fn get_exercise(&self, idx: u16) -> Exercise<'_> {
        Exercise {
            target: self.get_target(idx),
            name: self.get_name(idx),
            group: self.get_group(idx),
            needs_weight: self.needs_weights.contains(&idx),
        }
    }

    pub(crate) fn get_weight(&self, idx: u16) -> u8 {
        if self.needs_weights.contains(&idx) { self.weight } else { 0 }
    }

    pub(crate) fn get_name(&self, idx: u16) -> &str {
        self.buffer.get(self.names[usize::from(idx)])
    }

    pub(crate) fn get_group(&self, idx: u16) -> &str {
        self.buffer.get(self.groups[usize::from(idx)])
    }

    pub(crate) fn get_target(&self, idx: u16) -> Target {
        if idx < self.indices[1] {
            Target::Core
        } else if idx < self.indices[2] {
            Target::Lower
        } else {
            debug_assert!(idx < self.indices[3]);
            Target::Upper
        }
    }

    pub(crate) fn get_target_range(
        &self,
        target: Target,
    ) -> std::ops::Range<u16> {
        match target {
            Target::Core => self.indices[0]..self.indices[1],
            Target::Lower => self.indices[1]..self.indices[2],
            Target::Upper => self.indices[2]..self.indices[3],
        }
    }

    pub(crate) fn from_file(
        path: &Path,
        duration: Duration,
        weight: u8,
    ) -> eyre::Result<Self> {
        let mut file = File::open(path).wrap_err_with(|| {
            format!("Config file `{}` could not be read", path.display())
        })?;
        let mut buf = String::new();
        file.read_to_string(&mut buf)?;

        let value: ConfigBuilder =
            serde_json::from_str(&buf).wrap_err_with(|| {
                format!("Could not parse file `{}`", path.display())
            })?;
        let mut builder = BufferBuilder::new();
        let url = builder.insert(value.url);

        let mut indices = [0, 0, 0, 0];
        let capacity = value.specs.upper.len()
            + value.specs.lower.len()
            + value.specs.core.len();
        let mut groups: Vec<Index> = Vec::with_capacity(capacity);
        let mut names = Vec::with_capacity(capacity);
        let mut needs_weights = HashSet::new();

        process(
            &mut builder,
            value.specs.core,
            &mut groups,
            &mut names,
            &mut needs_weights,
        );
        debug_assert!(groups.len() <= u16::MAX.into());
        indices[1] = groups.len() as u16;

        process(
            &mut builder,
            value.specs.lower,
            &mut groups,
            &mut names,
            &mut needs_weights,
        );
        debug_assert!(groups.len() <= u16::MAX.into());
        indices[2] = groups.len() as u16;

        process(
            &mut builder,
            value.specs.upper,
            &mut groups,
            &mut names,
            &mut needs_weights,
        );
        debug_assert!(groups.len() <= u16::MAX.into());
        indices[3] = groups.len() as u16;

        Ok(Config {
            url,
            duration,
            weight,
            buffer: builder.into_buffer(),
            indices,
            groups: groups.into(),
            names: names.into(),
            needs_weights,
        })
    }
}

fn process<'a>(
    builder: &mut BufferBuilder<'a>,
    specs: HashMap<&'a str, Spec<'a>>,
    groups: &mut Vec<Index>,
    names: &mut Vec<Index>,
    weights: &mut HashSet<u16>,
) {
    for (k, v) in specs {
        let index = builder.insert(k);
        match v.variations {
            None => {
                groups.push(index);
                names.push(index);
                if v.needs_weight {
                    debug_assert!(groups.len() <= u16::MAX.into());
                    weights.insert(groups.len() as u16);
                }
            }
            Some(vars) => {
                for var in vars {
                    groups.push(index);
                    names.push(builder.insert(var));
                    if v.needs_weight {
                        debug_assert!(groups.len() <= u16::MAX.into());
                        weights.insert(groups.len() as u16);
                    }
                }
            }
        }
    }
}
