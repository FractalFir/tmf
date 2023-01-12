use std::io::{Read,BufReader,Result};
pub struct UnalignedReader<R:Read>{
    reader:BufReader<R>,
    last_byte:u8,
    consumed:u8,
}
impl<R:Read> UnalignedReader<R>{
    fn get_bits(&mut self,mut count:u8)->std::io::Result<u8>{
        if self.consumed >= 8{
            let mut tmp = [0];
            self.reader.read(&mut tmp)?;
            self.last_byte = tmp[0];
            self.consumed = 0;
        }
        let to_consume = count.min(8-self.consumed);     
        let mut res = if to_consume == 8{
            let res = self.last_byte;
            self.last_byte = 0;
            res
        }
        else{
            let res = self.last_byte>>(8-to_consume);
            self.last_byte = self.last_byte<<to_consume;
            res
        };
        self.consumed += to_consume;
        count -= to_consume;
        if count > 0{
            if self.consumed >= 8{
                let mut tmp = [0];
                self.reader.read(&mut tmp)?;
                self.last_byte = tmp[0];
                self.consumed = 0;
            }
            let to_consume = count.min(8-self.consumed);
            res = (res<<to_consume) | (self.last_byte>>(8-to_consume));
            self.last_byte = self.last_byte<<to_consume;
            self.consumed += to_consume;
        }
        Ok(res)
    }
    pub fn read_unaligned(&mut self,mode:UnalignedRWMode)->Result<u64>{
        let mut to_consume = mode.0;
        let mut res:u64 = 0;
        while to_consume > 0{
            let consuming = to_consume.min(8);
            res = res << consuming;
            let bits = self.get_bits(consuming)?;
            res += bits as u64;
            to_consume -= consuming;
        }
        Ok(res)
    }
    pub fn new(r:R)->Self{
        let reader = BufReader::new(r);
        let last_byte = 0;//read this
        let consumed = 8;
        Self{last_byte,consumed,reader}
    }
}
pub struct UnalignedRWMode(u8);
#[cfg(test)]
mod test_reader{
    use super::*;
    #[test]
    fn half_aligned_read(){
        let bytes:[u8;8] = [0x01,0x23,0x45,0x67,0x89,0xAB,0xCD,0xEF];
        let mut reader = UnalignedReader::new(&bytes as &[u8]);
        for byte in 0..0x10{
            let rbyte = reader.read_unaligned(UnalignedRWMode(4)).unwrap() as u8;
            assert!(rbyte == byte,"{rbyte} != {byte}");
        }
    }
    #[test]
    fn one_and_half_aligned_read(){
        let bytes:[u8;12] = [0x00,0x0F,0xFF,0xF0,0xFA,0xBC,0x1A,0x58,0x54,0x48,0x55,0x64]; 
        let expected:[u16;8] = [0x000,0xFFF,0xF0F,0xABC,0x1A5,0x854,0x485,0x564];
        let mut reader = UnalignedReader::new(&bytes as &[u8]);
        for byte in expected{
            let rbyte = reader.read_unaligned(UnalignedRWMode(12)).unwrap() as u16;
            assert!(rbyte == byte,"{rbyte} != {byte}");
        }
    }
    #[test]
    fn bin_3_aligned_read(){
        // 000 001 01|0 011 100 1|01 110 111
        let bytes:[u8;3] = [0b00000101,0b00111001,0b01110111];
        let mut reader = UnalignedReader::new(&bytes as &[u8]);
        for byte in 0..0x8{
            let rbyte = reader.read_unaligned(UnalignedRWMode(3)).unwrap() as u8;
            assert!(rbyte == byte,"{rbyte:8b} != {byte:8b}");
        }
    }
}
