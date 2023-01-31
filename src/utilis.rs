use crate::IndexType;
/// Fast method for pruning unused vertices. Finds the lowest used vertex and removes it. O(n), does **not** remove all unused vertices. Used mostly
/// in .obj loading. Fast because if not needed exits quickly.
pub(crate) fn fast_prune<T: Sized>(data: &mut Vec<T>, indices: &mut [IndexType]) {
    // Smallest index
    let mut min_index = IndexType::MAX;
    for index in indices.iter() {
        if *index == 0 {
            return;
        };
        min_index = min_index.min(*index);
    }
    data.drain(0..(min_index as usize));
    // Begin index rebasing
    for index in &mut *indices {
        *index -= min_index;
    }
}
