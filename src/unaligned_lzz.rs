use std::io::{Read,Write,Result}; 
// Implementation of the LLZ encoder optimized for efficient compression of UBA-s.
// This could be optimised using bitwise operations, but it is just a proff of concept, so this is delayed indefintly.
pub struct ULZZEncoder{
	window:Box<[bool]>,
	c_bits:u8,
}
fn bytes_to_bits(bytes:&[u8])->Box<[bool]>{
	let mut bits = Vec::with_capacity(bytes.len()*8);
	for byte in bytes{
		bits.push(byte&0b10000000 != 0);
		bits.push(byte&0b01000000 != 0);
		bits.push(byte&0b00100000 != 0);
		bits.push(byte&0b00010000 != 0);
		bits.push(byte&0b00001000 != 0);
		bits.push(byte&0b00000100 != 0);
		bits.push(byte&0b00000010 != 0);
		bits.push(byte&0b00000001 != 0);
	}
	bits.into()
}
fn bits_to_bytes(bits:&[bool])->Box<[u8]>{
	let mut bytes = Vec::with_capacity(bits.len()/8);
	for i in 0..(bits.len()/8){
		let byte = (bits[i*8] as u8)<<7 | (bits[i*8+1] as u8)<<6 | (bits[i*8+2] as u8)<<5
 		| (bits[i*8+3] as u8)<<4 | (bits[i*8+4] as u8)<<3 | (bits[i*8+5] as u8)<<2 | (bits[i*8+6] as u8)<<1
	 	| (bits[i*8+7] as u8);
		bytes.push(byte);
	}
	bytes.into()
}
// for each point in sliding window
// start comparing it and this sequence
// keep longest
fn get_compression(input:&[bool],curr_pos:usize,max_len:u8)->(usize,usize){
	// Not best found length, but maximum that can be read forward
	let forward_len = (input.len() - curr_pos).min(1<<max_len);
	let backward_beg = 0;
	let mut best_len = 0;
	let mut best_pos = 0;
	for i in backward_beg..curr_pos{
		for j in 0..forward_length{
			if input[curr_pos + j] != input[i]{
			
			}
		}	
	}
	return (best_len,best_pos);
}
pub fn encode_prec<R:Read,W:Write>(reader:&mut R,writer:&mut W,precision_bits:u8)->Result<()>{
	let mut bytes = Vec::with_capacity(4096);
	reader.read_to_end(&mut bytes)?;
	let bits = bytes_to_bits(&bytes);
	for i in 0..bits.len(){
		let curr = get_compression(&bits,i,precision_bits);
		println!("Compression at {i}: {curr:?}");
	}
	todo!();
}
#[cfg(test)]
mod ulzz_test{
	use super::*;
	const BITS:[bool;16] = [true,true,true,false,false,false,true,false,true,false,false,true,false,true,true,false];
	const BYTES:[u8;2] = [0b11100010,0b10010110];
	#[test]
	fn by_to_bi(){
		 let bits = bytes_to_bits(&BYTES);
		 assert!(&BITS == bits.as_ref(),"{BITS:?} != {:?}",bits.as_ref());
	}
	#[test]
	fn bi_to_by(){
		 let bytes = bits_to_bytes(&BITS);
		 println!("{:08b} {:08b}",bytes[0],BYTES[0]);
		 assert!(&BYTES == bytes.as_ref(),"{BYTES:?} != {:?}",bytes.as_ref());
	}
	const ENCODABLE:[u64;12] = [394993443,928483,3485839,39294354,33939429,33949349,3493959343993,88434,24244,242,323,13];
	#[test]
	fn test_encode(){
		let bytes:[u8;12*8] = unsafe{std::mem::transmute(ENCODABLE)};
		let mut res = Vec::new();
		encode_prec(&mut (&bytes as &[u8]),&mut res,5);
	}
}
