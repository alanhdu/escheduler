use std::{collections::HashMap, fs::File, io::Read, path::Path};

use serde::Deserialize;

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
pub(crate) struct Config {}

impl<'a> TryFrom<&'a Path> for Config {
    type Error = eyre::Error;

    fn try_from(value: &'a Path) -> Result<Self, Self::Error> {
        let mut file = File::open(value)?;
        let mut buf = String::new();
        file.read_to_string(&mut buf)?;

        let result: ConfigBuilder = serde_json::from_str(&buf)?;
        dbg!(&result);
        todo!();
    }
}
