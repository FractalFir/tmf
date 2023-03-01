// Compares two sequences of bits in (*a*,*b*) with lengths (*length_a*,*length_b*) offset to right by (*offset_a*,*offset_b*)
// Returns either the amount of bytes in sequence that match, or u64::BITS + 1(65) if entire sequence matches
fn cmp_seq(a: u64, offset_a: u8, length_a: u8, b: u64, offset_b: u8, length_b: u8) -> u32 {
    //println!("a: {a:064b}\nb: {a:064b}");
    let min_len = length_a.min(length_b);
    let a_bshift = u64::BITS as u8 - min_len - offset_a;
    let a = (a >> a_bshift) << (u64::BITS as u8 - min_len);
    let b_bshift = u64::BITS as u8 - min_len - offset_b;
    let b = (b >> b_bshift) << (u64::BITS as u8 - min_len);
    let match_len = (a ^ b).leading_zeros();
    if match_len >= min_len as u32 {
        u64::BITS + 1
    } else {
        match_len
    }
}
// compares sequences of bits starting at arbitrary location within the array.
fn get_seq_match(a: &[u64], b: &[u64], mut offset_a: u64, mut offset_b: u64) -> u64 {
    let mut eq_len = 0;
    while (offset_a / (u64::BITS as u64) < a.len() as u64)
        && (offset_b / (u64::BITS as u64) < b.len() as u64)
    {
        let index_a = (offset_a / (u64::BITS as u64)) as usize;
        let index_b = (offset_b / (u64::BITS as u64)) as usize;
        let loc_offset_a = (offset_a % (u64::BITS as u64)) as u8;
        let loc_offset_b = (offset_b % (u64::BITS as u64)) as u8;
        let loc_len_a = (u64::BITS as u8 - loc_offset_a) as u8;
        let loc_len_b = (u64::BITS as u8 - loc_offset_b) as u8;
        let l = cmp_seq(
            a[index_a],
            loc_offset_a,
            loc_len_a,
            b[index_b],
            loc_offset_b,
            loc_len_b,
        );
        if l != 65 {
            return eq_len + l as u64;
        } else {
            let l = loc_len_a.min(loc_len_b);
            offset_a += l as u64;
            offset_b += l as u64;
            eq_len += l as u64;
        }
    }
    eq_len
}
fn get_best_match_within_window(a:&[u64],b:&[u64],offset:u64,sliding_window:u8)->(u64,u64){
    let slide_beg = (offset as i64 - sliding_window as i64).max(0) as u64;
    let mut best_offset = 0;
    let mut best_length = 0;
    println!("slide_beg:{slide_beg}, offset:{offset}, sliding_window:{sliding_window}");
    for slide_offset in slide_beg..offset{
        let curr_len = get_seq_match(a,b,offset,slide_offset);
        println!("curr_len:{curr_len}");
        if curr_len > best_length{
            best_length = curr_len;
            best_offset = slide_offset;
        }
    }
    (best_offset,best_length)
}
fn encode(data:&[u64],bits:u64)->Vec<u64>{
    let sliding_window = 1<<bits;
    let end = (data.len() as u64)*(u64::BITS as u64);
    for curr_offset in 0..end{
        //let (o,l) = get_best_match_within_window()
    }
    todo!();
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
    let a: [u64; 2] = [
        0b0000000111110110111110110100001111100001100011101011001000000010,
        0b0110011011000010110001111010111111001100010010000001000011001000,
    ];
    let b: [u64; 2] = [
        0b0000000111110110111110110100001111100001100011101011001000000010,
        0b0110011011000010110001111010111111001100010010000001000011001000,
    ];
    assert!(get_seq_match(&a, &b, 0, 0) == 128);
    let b: [u64; 2] = [
        0b0000001111101101111101101000011111000011000111010110010000000100,
        0b1100110110000101100011110101111110011000100100000010000110010000,
    ];
    assert!(get_seq_match(&a, &b, 1, 0) == 127);
    assert!(get_seq_match(&b, &a, 0, 1) == 127);
}
#[test]
fn test_log_partial_eq(){
    let a: [u64; 2] = [
        0b0000000111110110111110110100001111100001100011101011001000000010,
        0b0110011011000010110001111010111111001100010010000001000011000000,
    ];
    let b: [u64; 2] = [
        0b0000000111110110111110110100001111100001100011101011001000000010,
        0b0110011011000010110001111010111111001100010010000001000011001000,
    ];
    assert!(get_seq_match(&a, &b, 0, 0) == 124);
     let b: [u64; 2] = [
        0b0000000111110110111110110100001111100001100011101011001001000010,
        0b0110011011000010110001111010111111001100010010000001000011000000,
    ];
    let smatch = get_seq_match(&a, &b, 0, 0);
    assert!(smatch == 57,"{smatch}");
}
#[test]
fn find_in_sliding_win(){
    let a: [u64; 2] = [
        0b0000000111110110111110110100001111100001100011101011001000000010,
        0b0110011011000010110001111010111111001100010010000001000011000000,
    ];
    let b: [u64; 2] = [
        0b0000000111110110111110110100001111100001100011101011001000000010,
        0b0110011011000010110001111010111111001100010010000001000011001000,
    ];
    let (o,l) = get_best_match_within_window(&a,&b,60,80);
    println!("o:{o},l:{l}");
    todo!();
}
