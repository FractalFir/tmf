/// Module used to handle reads of data which is not bit aligned(for example, 3 or 17 bits). This is the module that allows for heavy compression used in this format.
pub mod unaligned_rw;
pub fn add(left: usize, right: usize) -> usize {
    left + right
}

