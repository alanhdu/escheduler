use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;

type Brand<'id> = PhantomData<fn(&'id ()) -> &'id ()>;

pub(crate) struct Lifetime<'id>(Brand<'id>);

impl<'id> Lifetime<'id> {
    pub(crate) fn with_lifetime<R, F>(f: F) -> R
    where
        F: for<'a> FnOnce(Lifetime<'a>) -> R,
    {
        let brand = Lifetime(PhantomData);
        f(brand)
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub(crate) struct StrIndex<'id> {
    start: u16,
    stop: u16,
    _brand: Brand<'id>,
}

/// A bad string interner
#[derive(Debug)]
pub(crate) struct Buffer<'id> {
    buf: String,
    _brand: Brand<'id>,
}

impl<'id> Buffer<'id> {
    pub(crate) fn get(&self, idx: StrIndex<'id>) -> &str {
        &self.buf[(idx.start.into())..(idx.stop.into())]
    }
}

pub(crate) struct StringInterner<'a, 'id> {
    map: HashMap<&'a str, (u16, u16)>,
    buf: String,
    _brand: Brand<'id>,
}

impl<'a, 'id> StringInterner<'a, 'id> {
    pub(crate) fn with_lifetime(
        lifetime: Lifetime<'id>,
    ) -> StringInterner<'a, 'id> {
        StringInterner {
            map: HashMap::new(),
            buf: String::new(),
            _brand: PhantomData,
        }
    }

    pub(crate) fn insert(&mut self, key: &'a str) -> StrIndex<'id> {
        let (start, stop) = *self.map.entry(key).or_insert_with(|| {
            let start = self.buf.len() as u16;
            self.buf.push_str(key);
            debug_assert!(self.buf.len() <= u16::MAX.into());
            (start, self.buf.len() as u16)
        });
        StrIndex { start, stop, _brand: PhantomData }
    }

    pub(crate) fn into_buffer(self) -> Buffer<'id> {
        Buffer { buf: self.buf, _brand: PhantomData }
    }
}
