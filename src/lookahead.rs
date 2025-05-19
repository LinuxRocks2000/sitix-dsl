use std::collections::VecDeque;


pub trait LookaheadBuffer<T> {
    fn next(&mut self) -> Option<T>;
    fn peek(&mut self) -> Option<T>;
    fn get_head(&self) -> usize;

    fn skip(&mut self, skipper : impl Fn(T) -> bool) {
        while let Some(c) = self.peek() {
            if skipper(c) {
                self.next();
            }
            else {
                return;
            }
        }
    }
}

#[derive(Debug)]
pub struct SimpleLLBuffer<T, Iter : Iterator<Item = T>> {
    iterator : Iter,
    buffer : VecDeque<T>,
    head : usize
}

impl<T, Iter : Iterator<Item = T>> SimpleLLBuffer<T, Iter> {
    pub fn new(iterator : Iter) -> Self {
        Self {
            iterator,
            buffer : VecDeque::new(),
            head : 0
        }
    }
}

impl<T : Clone, Iter : Iterator<Item = T>> LookaheadBuffer<T> for SimpleLLBuffer<T, Iter> {
    fn next(&mut self) -> Option<T> {
        // if the buffer contains 0 elements, this just forwards to the iterator's next() function
        // if the buffer contains more than 0 elements, this pops off the first element and returns it
        self.head += 1;
        if self.buffer.len() > 0 {
            self.buffer.pop_front()
        }
        else {
            self.iterator.next()
        }
    }

    fn peek(&mut self) -> Option<T> {
        // if the buffer contains 0 elements, pull one from the iterator and append it to the back of the vecdeque.
        // then, return the first element.

        if self.buffer.len() == 0 {
            self.buffer.push_front(self.iterator.next()?);
        }
        self.buffer.get(0).cloned()
    }

    fn get_head(&self) -> usize {
        self.head
    }
}
