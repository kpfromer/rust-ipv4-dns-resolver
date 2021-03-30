use std::sync::{Condvar, Mutex};

use crate::ARRAY_SIZE;
use anyhow::{anyhow, Result};

#[derive(Debug)]
pub struct Buffer<T> {
    data: [Option<T>; ARRAY_SIZE],
    length: usize,
    pub running_producers: usize,
}

impl<T: Clone> Buffer<T> {
    pub fn new() -> Self {
        let array: [Option<T>; ARRAY_SIZE] = Default::default();
        Self {
            data: array,
            length: 0,
            running_producers: 0,
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.length == 0 {
            None
        } else {
            self.length -= 1;
            let value = self.data[self.length].clone();
            self.data[self.length] = None;

            value
        }
    }

    pub fn push(&mut self, value: T) -> Result<()> {
        if self.length == ARRAY_SIZE {
            Err(anyhow!("Buffer is full!"))
        } else {
            self.data[self.length] = Some(value);
            self.length += 1;
            Ok(())
        }
    }

    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    pub fn is_full(&self) -> bool {
        self.length == ARRAY_SIZE
    }
}

pub struct SharedBuffer<T> {
    pub buffer: Mutex<Buffer<T>>,
    pub can_consume: Condvar,
    pub can_produce: Condvar,
}

impl<T: Clone> SharedBuffer<T> {
    pub fn new() -> Self {
        Self {
            buffer: Mutex::new(Buffer::<T>::new()),
            can_consume: Condvar::new(),
            can_produce: Condvar::new(),
        }
    }
}
