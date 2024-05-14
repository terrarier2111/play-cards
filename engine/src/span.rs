#[derive(Clone, Copy, Debug)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    #[inline]
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    #[inline]
    pub fn new_single(start: usize) -> Self {
        Self {
            start,
            end: start + 1,
        }
    }

    #[inline]
    pub fn len(self) -> usize {
        self.end - self.start
    }
}
