// Module not completed yet
#![allow(dead_code)]
type CompressionType = u64;
// Compares two sequences of bits in (*a*,*b*) with lengths (*length_a*,*length_b*) offset to right by (*offset_a*,*offset_b*)
// Returns either the amount of bytes in sequence that match, or CompressionType::BITS + 1(65) if entire sequence matches
fn cmp_seq(
    a: CompressionType,
    offset_a: u8,
    length_a: u8,
    b: CompressionType,
    offset_b: u8,
    length_b: u8,
) -> u32 {
    //println!("a: {a:064b}\nb: {a:064b}");
    let min_len = length_a.min(length_b);
    let a_bshift = CompressionType::BITS as u8 - min_len - offset_a;
    let a = (a >> a_bshift) << (CompressionType::BITS as u8 - min_len);
    let b_bshift = CompressionType::BITS as u8 - min_len - offset_b;
    let b = (b >> b_bshift) << (CompressionType::BITS as u8 - min_len);
    let match_len = (a ^ b).leading_zeros();
    if match_len >= min_len as u32 {
        CompressionType::BITS + 1
    } else {
        match_len
    }
}
// compares sequences of bits starting at arbitrary location within the array.
fn get_seq_match(
    a: &[CompressionType],
    b: &[CompressionType],
    mut offset_a: CompressionType,
    mut offset_b: CompressionType,
) -> CompressionType {
    let mut eq_len = 0;
    while (offset_a / (CompressionType::BITS as CompressionType) < a.len() as CompressionType)
        && (offset_b / (CompressionType::BITS as CompressionType) < b.len() as CompressionType)
    {
        let index_a = (offset_a / (CompressionType::BITS as CompressionType)) as usize;
        let index_b = (offset_b / (CompressionType::BITS as CompressionType)) as usize;
        let loc_offset_a = (offset_a % (CompressionType::BITS as CompressionType)) as u8;
        let loc_offset_b = (offset_b % (CompressionType::BITS as CompressionType)) as u8;
        let loc_len_a = CompressionType::BITS as u8 - loc_offset_a;
        let loc_len_b = CompressionType::BITS as u8 - loc_offset_b;
        let l = cmp_seq(
            a[index_a],
            loc_offset_a,
            loc_len_a,
            b[index_b],
            loc_offset_b,
            loc_len_b,
        );
        if l != 65 {
            return eq_len + l as CompressionType;
        } else {
            let l = loc_len_a.min(loc_len_b);
            offset_a += l as CompressionType;
            offset_b += l as CompressionType;
            eq_len += l as CompressionType;
        }
    }
    eq_len
}
fn get_best_match_within_window(
    a: &[CompressionType],
    b: &[CompressionType],
    offset: CompressionType,
    sliding_window: u8,
) -> (CompressionType, CompressionType) {
    let slide_beg = (offset as i64 - sliding_window as i64).max(0) as CompressionType;
    let mut best_offset = 0;
    let mut best_length = 0;
    //println!("slide_beg:{slide_beg}, offset:{offset}, sliding_window:{sliding_window}");
    for slide_offset in slide_beg..offset {
        let curr_len = get_seq_match(a, b, offset, slide_offset);
        //println!("curr_len:{curr_len}");
        if curr_len > best_length {
            best_length = curr_len;
            best_offset = slide_offset;
        }
    }
    (best_offset, best_length)
}
fn eval_match(len: CompressionType, bits_len: u8) -> bool {
    let nocompress_len = (bits_len as CompressionType) * 2 + 1;
    if len >= nocompress_len {
        return true;
    }
    //Additional compression gain coming from smaller segment count.
    let compress_gain =
        ((len as f32) / ((1 << bits_len) as f32) * (nocompress_len as f32)) as CompressionType;
    len > compress_gain + nocompress_len
}
use crate::unaligned_rw::*;
const SEG_TYPE_PREC: UnalignedRWMode = UnalignedRWMode::precision_bits(1);
const SEG_UNCOMPRESSED: u8 = 0;
const SEG_COMPRESSED: u8 = 1;
fn encode(data: &[CompressionType], bits: u8) -> Vec<CompressionType> {
    let sliding_window = 1 << bits;
    let end = (data.len() as CompressionType) * (CompressionType::BITS as CompressionType);

    let mut curr_offset = 0;
    let mut unc_seg_len = 0;
    let mut unc_beg = 0;

    let mut compressed = Vec::with_capacity(data.len());
    let mut compression = UnalignedWriter::new(&mut compressed);
    let offset_prec = UnalignedRWMode::precision_bits(bits);
    while curr_offset < end {
        let (o, l) = get_best_match_within_window(data, data, curr_offset, sliding_window);
        #[allow(clippy::unnecessary_cast)]
        if eval_match(l, bits) {
            println!("Favorable match of length:{l} at offset:{curr_offset}, flowing uncompressed segment of length:{unc_seg_len} starting at:{unc_beg}");

            compression
                .write_unaligned(SEG_TYPE_PREC, SEG_UNCOMPRESSED as u64)
                .expect("TODO: handle this");
            compression
                .write_unaligned(offset_prec, unc_seg_len as u64)
                .expect("TODO: handle this");
            println!("TODO:write the uncompressed segment!");

            compression
                .write_unaligned(SEG_TYPE_PREC, SEG_COMPRESSED as u64)
                .expect("TODO: handle this");
            compression
                .write_unaligned(offset_prec, l as u64)
                .expect("TODO: handle this");
            compression
                .write_unaligned(offset_prec, (curr_offset - o) as u64)
                .expect("TODO: handle this");

            curr_offset += l;
            unc_seg_len = 0;
            unc_beg = curr_offset;
        } else {
            //println!("o:{o},l:{l}");
            curr_offset += 1;
            unc_seg_len += 1;
        }
    }
    if unc_seg_len > 0 {
        println!("TODO:write the last uncompressed segment!");
    }
    todo!();
}
fn bsl_safe(val: CompressionType, shift: CompressionType) -> CompressionType {
    if shift == 0 {
        val
    } else {
        val << shift
    }
}
fn bsr_safe(val: CompressionType, shift: CompressionType) -> CompressionType {
    if shift == 0 {
        val
    } else {
        val >> shift
    }
}
// Probably needs a complete rewrite
fn copy_bits_in_byte(
    src: CompressionType,
    target: CompressionType,
    src_offset: CompressionType,
    target_offset: CompressionType,
    cpy_len: CompressionType,
) -> CompressionType {
    debug_assert!(cpy_len != 0);
    let src = src << src_offset;
    let src = src >> (CompressionType::BITS as CompressionType - cpy_len);
    target | src << (CompressionType::BITS as CompressionType - target_offset)
}
fn bwise_memcpy(
    src: &[CompressionType],
    mut src_offset: CompressionType,
    mut target_offset: CompressionType,
    mut length: CompressionType,
    target: &mut [CompressionType],
) {
    let mut target_byte =
        target[(target_offset / (CompressionType::BITS as CompressionType)) as usize];
    while length > 0 {
        let next_src_offset = ((src_offset / (CompressionType::BITS as CompressionType - 1)) + 1)
            * (CompressionType::BITS as CompressionType);
        let next_target_offset = ((src_offset / (CompressionType::BITS as CompressionType - 1))
            + 1)
            * (CompressionType::BITS as CompressionType);
        let curr_cpy_length = (next_src_offset - src_offset)
            .min(next_target_offset - target_offset)
            .min(length);

        assert!(length > 0);

        let src_byte = src[(src_offset / (CompressionType::BITS as CompressionType)) as usize];
        let curr_src_offset = src_offset % (CompressionType::BITS as CompressionType);
        let curr_target_offset = target_offset % (CompressionType::BITS as CompressionType);
        // target byte: | src_offset | target_data | CompressionType::BITS - curr_cpy_length | -> |0_padding|target_data|
        // |
        let src_byte = bsr_safe(
            bsl_safe(src_byte, curr_src_offset),
            CompressionType::BITS as CompressionType - curr_cpy_length + curr_src_offset,
        );
        println!("src_byte: 0b{src_byte:064b}, target_byte:{target_byte:064b}");
        println!("curr_target_offset{curr_target_offset},curr_cpy_length{curr_cpy_length}");
        let src_byte = bsl_safe(
            src_byte,
            CompressionType::BITS as CompressionType - curr_target_offset - curr_cpy_length,
        );
        target_byte |= src_byte;
        println!("src_byte: 0b{src_byte:064b}, target_byte:{target_byte:064b}");
        let target_written = curr_target_offset + curr_cpy_length;
        println!("target_written:{target_written}");
        if target_written >= CompressionType::BITS as CompressionType {
            println!("writing to traget!");
            target[(target_offset / (CompressionType::BITS as CompressionType)) as usize] =
                target_byte;
            target_byte = 0;
        }
        length -= curr_cpy_length;
        target_offset += curr_cpy_length;
        src_offset += curr_cpy_length;
        println!("src_offset:{src_offset}, next_src_offset:{next_src_offset},next_target_offset:{next_target_offset},curr_cpy_length:{curr_cpy_length}");
    }
    todo!("Coping not implemented yet, but target is:{target:?}!");
}
//Not finished yet!
#[test]
#[ignore]
fn test_bwise_cpy() {
    let a: [CompressionType; 2] = [
        0b0000000111110110111110110100001111100001100011101011001000000010,
        0b0110011011000010110001111010111111001100010010000001000011000000,
    ];
    let mut b = vec![0; 2];
    //let mut b_writer = UnalignedWriter::new(&mut b);
    bwise_memcpy(&a, 8, 0, 70, &mut b);
    assert!(get_seq_match(&a, &b, 8, 0) == 70);
}
#[test]
fn test_equ_len_no_offset() {
    let a = 0b0000000111110110111110110100001111100001100011101011001000000011;
    let b = 0b0000000111110110111110110100001111100001100011101011001000000011 - 4294967296;
    let match_len = cmp_seq(a, 0, 64, b, 0, 64);
    assert!(match_len == 31, "{match_len:?} != 31");
    println!("STAGE 1.");
    let a = 0b0000000111110110111110110100001111100001100011101011001000000011;
    let b = 0b1000000111110110111110110100001111100001100011101011001000000011;
    let match_len = cmp_seq(a, 0, 64, b, 0, 64);
    assert!(match_len == 0, "{match_len:?} != 0");
    println!("STAGE 2.");
}

#[test]
fn test_diff_len() {
    let a = 0b0000000111110110111110110100001111100001100011101011001000000011;
    let b = 0b0000000111110110111110110100001111100001100011101011001000000011 - 2147483648;
    let match_len = cmp_seq(a, 0, 30, b, 0, 30);
    assert!(match_len == 65, "{match_len:?} != 65");
    println!("STAGE 1.");
    let a = 0b0000000111110110111110110100001111100001100011101011001000000011;
    let b = 0b0000000111110110111110110100001111100001100011101011001000000011 - 2147483648;
    let match_len = cmp_seq(a, 0, 64, b, 0, 33);
    assert!(match_len == 32, "{match_len:?} != 32");
    println!("STAGE 2.");
    let a = 0b0000000111110110111110110100001111100001100011101011001000000011;
    let b = 0b0000000111110110111110110100001111100001100011101011001000000011 - 2147483648;
    let match_len = cmp_seq(a, 0, 33, b, 0, 55);
    assert!(match_len == 32, "{match_len:?} != 32");
    println!("STAGE 3.");
}
#[test]
fn test_diff_offset() {
    let a = 0b0000000111110110111110110100001111100001100011101011001000000011;
    let b = 0b0001111101101111101101000011111000011000111010110010000000110000;
    let match_len = cmp_seq(a, 4, 36, b, 0, 36);
    assert!(match_len == 65, "{match_len:?} != 32");
    println!("STAGE 1.");
}
#[test]
fn test_long_full_eq() {
    let a: [CompressionType; 2] = [
        0b0000000111110110111110110100001111100001100011101011001000000010,
        0b0110011011000010110001111010111111001100010010000001000011001000,
    ];
    let b: [CompressionType; 2] = [
        0b0000000111110110111110110100001111100001100011101011001000000010,
        0b0110011011000010110001111010111111001100010010000001000011001000,
    ];
    assert!(get_seq_match(&a, &b, 0, 0) == 128);
    let b: [CompressionType; 2] = [
        0b0000001111101101111101101000011111000011000111010110010000000100,
        0b1100110110000101100011110101111110011000100100000010000110010000,
    ];
    assert!(get_seq_match(&a, &b, 1, 0) == 127);
    assert!(get_seq_match(&b, &a, 0, 1) == 127);
}
#[test]
fn test_log_partial_eq() {
    let a: [CompressionType; 2] = [
        0b0000000111110110111110110100001111100001100011101011001000000010,
        0b0110011011000010110001111010111111001100010010000001000011000000,
    ];
    let b: [CompressionType; 2] = [
        0b0000000111110110111110110100001111100001100011101011001000000010,
        0b0110011011000010110001111010111111001100010010000001000011001000,
    ];
    assert!(get_seq_match(&a, &b, 0, 0) == 124);
    let b: [CompressionType; 2] = [
        0b0000000111110110111110110100001111100001100011101011001001000010,
        0b0110011011000010110001111010111111001100010010000001000011000000,
    ];
    let smatch = get_seq_match(&a, &b, 0, 0);
    assert!(smatch == 57, "{smatch}");
}
#[test]
fn find_in_sliding_win() {
    let a: [CompressionType; 2] = [
        0b0000000111110110111110110100001111100001100011101011001000000010,
        0b0110011011000010110001111010111111001100010010000001000011000000,
    ];
    let b: [CompressionType; 2] = [
        0b0000000111110110111110110100001111100001100011101011001000000010,
        0b0110011011000010110001111010111111001100010010000001000011001000,
    ];
    let (o, l) = get_best_match_within_window(&a, &b, 60, 80);
}
#[test]
#[ignore]
fn test_encode() {
    let data = [2582, 8907545, 86606, 70115397, 717606];
    let compressed = encode(&data, 6);
}
