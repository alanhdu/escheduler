#![allow(unused)]
use std::collections::VecDeque;

pub(crate) struct Spec {
    idx: u16,
    best: u16,
    weight: u8,
}

pub(crate) struct Queue {
    queue: VecDeque<Spec>,
}
