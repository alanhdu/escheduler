use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::Read,
    path::Path,
};

use eyre::Context;
use serde::Deserialize;

use crate::buffer::{Buffer, BufferBuilder, Index};

#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Target {
    Core,
    Lower,
    Upper,
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
    buffer: Buffer,
    indices: [u16; 4],

    names: Box<[Index]>,
    groups: Box<[Index]>,
    needs_weights: HashSet<u16>,

    url: Index,
}

impl Config {
    pub(crate) fn from_file(path: &Path) -> eyre::Result<Self> {
        let mut file = File::open(path).wrap_err_with(|| {
            format!("Could not open file `{}`", path.display())
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
            buffer: builder.into_buffer(),
            groups: groups.into(),
            indices,
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
