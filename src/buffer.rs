use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub(crate) struct Index {
    start: u16,
    stop: u16,
}

#[derive(Debug)]
pub(crate) struct Buffer {
    buf: String,
}

impl Buffer {
    pub(crate) fn get(&self, idx: Index) -> &str {
        &self.buf[(idx.start.into())..(idx.stop.into())]
    }
}

pub(crate) struct BufferBuilder<'a> {
    map: HashMap<&'a str, (u16, u16)>,
    buf: String,
}

impl<'a> BufferBuilder<'a> {
    pub(crate) fn new() -> Self {
        BufferBuilder { map: HashMap::new(), buf: String::new() }
    }

    pub(crate) fn insert(&mut self, key: &'a str) -> Index {
        let (start, stop) = *self.map.entry(key).or_insert_with(|| {
            let start = self.buf.len() as u16;
            self.buf.push_str(key);
            debug_assert!(self.buf.len() <= u16::MAX.into());
            (start, self.buf.len() as u16)
        });
        Index { start, stop }
    }

    pub(crate) fn into_buffer(self) -> Buffer {
        Buffer { buf: self.buf }
    }
}
