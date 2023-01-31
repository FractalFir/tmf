use std::io::{Read, Result, Write};
// Implementation of the LZ encoder optimized for efficient compression of UBA-s.
// This could be optimised using bitwise operations, but it is just a proff of concept, so this is delayed indefintly.
pub struct UlzEncoder {
    window: Box<[bool]>,
    c_bits: u8,
}
fn bytes_to_bits(bytes: &[u8]) -> Box<[bool]> {
    let mut bits = Vec::with_capacity(bytes.len() * 8);
    for byte in bytes {
        bits.push(byte & 0b1000_0000 != 0);
        bits.push(byte & 0b0100_0000 != 0);
        bits.push(byte & 0b0010_0000 != 0);
        bits.push(byte & 0b0001_0000 != 0);
        bits.push(byte & 0b0000_1000 != 0);
        bits.push(byte & 0b0000_0100 != 0);
        bits.push(byte & 0b0000_0010 != 0);
        bits.push(byte & 0b0000_0001 != 0);
    }
    bits.into()
}
fn bits_to_bytes(bits: &[bool]) -> Box<[u8]> {
    let mut bytes = Vec::with_capacity(bits.len() / 8);
    for i in 0..(bits.len() / 8) {
        let byte = (bits[i * 8] as u8) << 7
            | (bits[i * 8 + 1] as u8) << 6
            | (bits[i * 8 + 2] as u8) << 5
            | (bits[i * 8 + 3] as u8) << 4
            | (bits[i * 8 + 4] as u8) << 3
            | (bits[i * 8 + 5] as u8) << 2
            | (bits[i * 8 + 6] as u8) << 1
            | (bits[i * 8 + 7] as u8);
        bytes.push(byte);
    }
    bytes.into()
}
// Evaluate length of LZ match
fn evaluate_match(input: &[bool], mut curr_pos: usize, mut match_beg: usize) -> usize {
    let mut len = 0;
    // Limit length
    while curr_pos < input.len() && match_beg < input.len() {
        if input[curr_pos] != input[match_beg] {
            break;
        };
        len += 1;
        curr_pos += 1;
        match_beg += 1;
    }
    len
}
// for each point in sliding window
// start comparing it and this sequence
// keep longest
fn find_best_match(input: &[bool], curr_pos: usize, max_len: u8) -> (usize, usize) {
    // Maximum number of bytes that can be read forward.
    let forward_len = (input.len() - curr_pos).min(1 << (max_len - 1));
    // Index of the backward most byte in the sliding window.
    let backward_beg = (0.max(curr_pos as isize - (1 << (max_len - 1)))) as usize;
    // Best length found so far
    let mut best_len: usize = 0;
    // Best position found so far
    let mut best_pos = 0;
    // For each point in sliding window
    for i in backward_beg..curr_pos {
        // Evaluate how good this match is
        let curr_max = evaluate_match(input, curr_pos, i);
        // If found match better than previous match, set the best match to current match.
        if curr_max > best_len {
            best_len = curr_max;
            best_pos = i;
        }
    }
    // return best found match
    return (best_len, best_pos);
}
pub fn encode_prec<R: Read, W: Write>(
    reader: &mut R,
    writer: &mut W,
    precision_bits: u8,
) -> Result<()> {
    let mut bytes = Vec::with_capacity(4096);
    reader.read_to_end(&mut bytes)?;
    let bits = bytes_to_bits(&bytes);
    for i in 0..bits.len() {
        let curr = find_best_match(&bits, i, precision_bits);
        if curr.0 > (precision_bits * 2) as usize {
            for j in 0..(curr.0) {
                println!("{} {}", bits[i], bits[curr.1 + j]);
                if bits[i] != bits[i + j] {
                    panic!()
                };
            }
            println!("Compression at {i}: {curr:?}");
        }
    }
    todo!();
}
#[cfg(test)]
mod ulz_test {
    use super::*;
    const BITS: [bool; 16] = [
        true, true, true, false, false, false, true, false, true, false, false, true, false, true,
        true, false,
    ];
    const BYTES: [u8; 2] = [0b11100010, 0b10010110];
    #[test]
    fn by_to_bi() {
        let bits = bytes_to_bits(&BYTES);
        assert!(&BITS == bits.as_ref(), "{BITS:?} != {:?}", bits.as_ref());
    }
    #[test]
    fn bi_to_by() {
        let bytes = bits_to_bytes(&BITS);
        println!("{:08b} {:08b}", bytes[0], BYTES[0]);
        assert!(
            &BYTES == bytes.as_ref(),
            "{BYTES:?} != {:?}",
            bytes.as_ref()
        );
    }
    const ENCODABLE: [u64; 12] = [
        394993443,
        928483,
        3485839,
        39294354,
        33939429,
        33949349,
        3493959343993,
        88434,
        24244,
        242,
        323,
        13,
    ];
    #[ignore]
    #[test]
    fn test_encode() {
        let bytes: [u8; 12 * 8] = unsafe { std::mem::transmute(ENCODABLE) };
        let mut res = Vec::new();
        encode_prec(&mut (&bytes as &[u8]), &mut res, 6);
    }
    #[ignore]
    #[test]
    fn test_evaluate_match() {
        let a = [false, false, false, false, true];
        let expected = [5, 3, 2, 1];
        for i in 0..a.len() {
            assert!(evaluate_match(&a, i, 0) == expected[i]);
        }
        todo!();
    }
}
