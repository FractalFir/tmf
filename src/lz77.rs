// Compares two sequences of bits in (*a*,*b*) with lengths (*length_a*,*length_b*) offset to right by (*offset_a*,*offset_b*)
// Returns either the amount of bytes in sequence that match 
fn cmp_seq(a:u64,offset_a:u8,length_a:u8,b:u64,offset_b:u8,length_b:u8)->u32{
    let offset_a:u8 = (length_a as i8 - length_b as i8).max(0) as u8 + offset_a;
    let offset_b:u8 = (length_b as i8 - length_a as i8).max(0) as u8 + offset_b;
    println!("offset_a:{offset_a}");
    let a = if offset_a > 0 {(a>>offset_a)<<(u64::BITS as u8 - offset_a)}else{a};
    let b = if offset_b > 0 {(b>>offset_b)<<(u64::BITS as u8 - offset_b)}else{b};
    let match_len = (a^b).leading_zeros();
    let max_len = length_a.max(length_b);
    match_len
}
// compares sequences of bits starting at arbitrary location within the array.
fn get_seq_len(){}
#[test]
fn test_equ_len_no_offset(){
    let a = 0b0000000111110110111110110100001111100001100011101011001000000011;
    let b = 0b0000000111110110111110110100001111100001100011101011001000000011-4294967296;
    let match_len = cmp_seq(a,0,64,b,0,64);
    assert!(match_len == 31,"{match_len:?} != 31");
    let a = 0b0000000111110110111110110100001111100001100011101011001000000011;
    let b = 0b1000000111110110111110110100001111100001100011101011001000000011;
    let match_len = cmp_seq(a,0,64,b,0,64);
    assert!(match_len == 0,"{match_len:?} != 0");
    let a = 0b0000000111110110111110110100001111100001100011101011001000000011;
    let b = 0b0100000111110110111110110100001111100001100011101011001000000011;
    let match_len = cmp_seq(a,0,64,b,0,64);
    assert!(match_len == 1,"{match_len:?} != 1");
    let a = 0b0000000111110110111110110100001111100001100011101011001000000011;
    let b = 0b0000000111110110111110110100001111100001100011101011001000000010;
    let match_len = cmp_seq(a,0,64,b,0,64);
    assert!(match_len == 63,"{match_len:?} != 63");
    let a = 0b0000000111110110111110110100001111100001100011101011001000000011;
    let b = 0b0000010111010110111110110100001011100001100011101001001000000010;
    let match_len = cmp_seq(a,0,64,b,0,64);
    assert!(match_len == 5,"{match_len:?} != 5");
    //This test is buggy. It currently shold fail?
    let a = 0b0000000111110110111110110100001111100001100011101011001000000011;
    let b = 0b0000000111110110111110110100001111100001100011101011001000000011-4294967296;
    let match_len = cmp_seq(a,0,40,b,0,40);
    assert!(match_len == 31,"{match_len:?} != 31");
    
    let a = 0b0000000111110110111110110100001111100001100011101011001000000011;
    let b = 0b0000000111110110111110110100001111100001100011101011001000000011-4294967296;
    let match_len = cmp_seq(a,0,1,b,0,1);
    assert!(match_len == 31,"{match_len:?} != 31");
}

#[test]
fn test_equ_len_offset(){
    let a = 0b0000000111110110111110110100001111100001100011101011001000000011;
    let b = 0b0000000111110110111110110100001111100001100011101011001000000011-2147483648;
    let match_len = cmp_seq(a,0,64,b,1,63);
    assert!(match_len == 31,"{match_len:?} != 31");
    
}
