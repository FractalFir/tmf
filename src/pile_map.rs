// Module not completed yet
#![allow(dead_code)]
/// Kinda like hash map, but is contiguous, uses simple indices and can be easily serialised
struct PileMap<T: PartialEq> {
    pile: Vec<T>,
}
impl<T: PartialEq + 'static> PileMap<T> {
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            pile: Vec::with_capacity(cap),
        }
    }
    pub fn new() -> Self {
        Self::with_capacity(16)
    }
    pub fn push(&mut self, t: T) -> usize {
        let mut index = 0;
        for curr in &self.pile {
            if curr.eq(&t) {
                return index;
            }
            index += 1;
        }
        self.pile.push(t);
        index + 1
    }
}
impl<T: PartialEq> From<PileMap<T>> for Vec<T> {
  fn from(val: PileMap<T>) -> Self {
         val.pile

    }
}
