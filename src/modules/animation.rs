pub struct Animation<T> {
    tick: usize,
    animation: Vec<T>,
}

impl<T> Animation<T> {
    pub fn new(animation: Vec<T>) -> Self {
        Self { tick: 0, animation }
    }

    pub fn next(&mut self) {
        if self.tick == self.animation.len() - 1 {
            self.tick = 0;
        } else {
            self.tick = self.tick + 1;
        }
    }

    pub fn current(&self) -> &T {
        self.animation.get(self.tick).unwrap()
    }

    pub fn reset(&mut self) {
        self.tick = 0;
    }
}
