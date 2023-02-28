// Compares two sequences of bits in (*a*,*b*) with lengths (*length_a*,*length_b*) offset to right by (*offset_a*,*offset_b*)
// Returns either the amount of bytes in sequence that match, or u64::BITS + 1(65) if entire sequence matches
fn cmp_seq(a:u64,offset_a:u8,length_a:u8,b:u64,offset_b:u8,length_b:u8)->u32{
    //println!("a: {a:064b}\nb: {a:064b}");
    let min_len = length_a.min(length_b);
    let a_bshift = (u64::BITS as u8 - min_len - offset_a);
    let a = (a>>a_bshift)<<(u64::BITS as u8 - min_len);
    let b_bshift = (u64::BITS as u8 - min_len - offset_b);
    let b = (b>>b_bshift)<<(u64::BITS as u8 - min_len);
    //println!("a: {a:064b}\nb: {a:064b}");
    let match_len = (a^b).leading_zeros();
    if match_len > min_len as u32 {
        u64::BITS + 1
    }
    else{
        match_len
    }  
}
// compares sequences of bits starting at arbitrary location within the array.
fn get_seq_len(){}
#[test]
fn test_equ_len_no_offset(){
    let a = 0b0000000111110110111110110100001111100001100011101011001000000011;
    let b = 0b0000000111110110111110110100001111100001100011101011001000000011-4294967296;
    let match_len = cmp_seq(a,0,64,b,0,64);
    assert!(match_len == 31,"{match_len:?} != 31");
    println!("STAGE 1.");
    let a = 0b0000000111110110111110110100001111100001100011101011001000000011;
    let b = 0b1000000111110110111110110100001111100001100011101011001000000011;
    let match_len = cmp_seq(a,0,64,b,0,64);
    assert!(match_len == 0,"{match_len:?} != 0");
    println!("STAGE 2.");
}

#[test]
fn test_diff_len(){
    let a = 0b0000000111110110111110110100001111100001100011101011001000000011;
    let b = 0b0000000111110110111110110100001111100001100011101011001000000011-2147483648;
    let match_len = cmp_seq(a,0,30,b,0,30);
    assert!(match_len == 65,"{match_len:?} != 65");
    println!("STAGE 1.");
    let a = 0b0000000111110110111110110100001111100001100011101011001000000011;
    let b = 0b0000000111110110111110110100001111100001100011101011001000000011-2147483648;
    let match_len = cmp_seq(a,0,64,b,0,33);
    assert!(match_len == 32,"{match_len:?} != 32");
    println!("STAGE 2.");
    let a = 0b0000000111110110111110110100001111100001100011101011001000000011;
    let b = 0b0000000111110110111110110100001111100001100011101011001000000011-2147483648;
    let match_len = cmp_seq(a,0,33,b,0,55);
    assert!(match_len == 32,"{match_len:?} != 32");
    println!("STAGE 3.");
}
#[test]
fn test_diff_offset(){
    let a = 0b0000000111110110111110110100001111100001100011101011001000000011;
    let b = 0b0001111101101111101101000011111000011000111010110010000000110000;
    let match_len = cmp_seq(a,4,36,b,0,36);
    assert!(match_len == 65,"{match_len:?} != 32");
    println!("STAGE 1.");
}
