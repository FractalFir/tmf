use std::io::{Read,BufReader,Write,BufWriter,Result};
pub struct UnalignedReader<R:Read>{
    reader:BufReader<R>,
    last_byte:u8,
    readen:u8,
}
impl<R:Read> UnalignedReader<R>{
    ///Reads *mode.0* bytes from self, keeping internal alighment
    pub fn read_unaligned(&mut self,mode:UnalignedRWMode)->Result<u64>{
        let mut res:u64 = 0;
        let mut total_read = mode.0;
        while total_read > 0{
            if self.readen >= 8{
                let mut tmp:[u8;1] = [0];
                self.reader.read(&mut tmp)?;
                self.last_byte = tmp[0];
                self.readen = 0;
            }
            let read_ammount = total_read.min(8 - self.readen);
            let read_offset = 8 - read_ammount;
            println!("read_offset:{read_offset}");
            res <<= read_ammount;
            println!("res:{res}");
            res |= (self.last_byte>>read_offset) as u64;
            self.readen += read_ammount;
            if read_ammount < 8{
                 self.last_byte <<= read_ammount;
            }
            total_read -= read_ammount;
        }
        Ok(res)
    }
    ///Creates new Unaligned Reader form *r* 
    pub fn new(r:R)->Self{
        let reader = BufReader::new(r);
        let last_byte = 0;//read this
        let readen = 8;
        Self{last_byte,readen,reader}
    }
}
pub struct UnalignedWriter<W:Write>{
    writer:BufWriter<W>,
    last_byte:u8,
    written:u8,
}
impl<W:Write> Drop for UnalignedWriter<W>{
    fn drop(&mut self){
        if self.written > 0{self.writer.write_all(&[self.last_byte]).expect("Unaligned writer failed to write last byte when dropped")};
        self.writer.flush().expect("Unaligned writer failed to flush when dropped");
    }
}
impl<W:Write> UnalignedWriter<W>{
    pub fn new(w:W)->Self{
        let writer = BufWriter::new(w);
        let last_byte = 0;
        let written = 0;
        Self{last_byte,written,writer}
    }
    pub fn write_unaligned(&mut self,mode:UnalignedRWMode,mut data:u64)->Result<()>{
        let mut total_write = mode.0;
        while total_write > 0{
            let write_ammount = total_write.min(8 - self.written);
            let write_offset = 8 - write_ammount;
            self.last_byte = if write_ammount == 8{
                ((data%256) as u8)
            }else{
                 self.last_byte | (((data%256) as u8)>>write_ammount)
            };
            println!("write_ammount:{write_ammount}");
            self.written += write_ammount;
            data = data<<write_ammount;
            if(self.written >= 8){
                println!("writing:{}",self.last_byte);
                self.writer.write(&[self.last_byte])?;
                self.written = 0;
                self.last_byte = 0;
            }
            total_write -= write_ammount;
        }
        Ok(())
    }
}
pub struct UnalignedRWMode(u8);
#[cfg(test)]
mod test_reader{
    use super::*;
    #[test]
    fn half_aligned(){
        let bytes:[u8;8] = [0x01,0x23,0x45,0x67,0x89,0xAB,0xCD,0xEF];
        let mut reader = UnalignedReader::new(&bytes as &[u8]);
        for byte in 0..0x10{
            let rbyte = reader.read_unaligned(UnalignedRWMode(4)).unwrap() as u8;
            //println!("{rbyte} != {byte}");
            assert!(rbyte == byte,"{rbyte} != {byte}");
        }
    }
    #[test]
    fn one_and_half_aligned(){
        let bytes:[u8;12] = [0x00,0x0F,0xFF,0xF0,0xFA,0xBC,0x1A,0x58,0x54,0x48,0x55,0x64]; 
        let expected:[u16;8] = [0x000,0xFFF,0xF0F,0xABC,0x1A5,0x854,0x485,0x564];
        let mut reader = UnalignedReader::new(&bytes as &[u8]);
        for byte in expected{
            let rbyte = reader.read_unaligned(UnalignedRWMode(12)).unwrap() as u16;
            assert!(rbyte == byte,"{rbyte} != {byte}");
        }
    }
    #[test]
    fn bin_3_aligned(){
        // 000 001 01|0 011 100 1|01 110 111
        let bytes:[u8;3] = [0b00000101,0b00111001,0b01110111];
        let mut reader = UnalignedReader::new(&bytes as &[u8]);
        for byte in 0..0x8{
            let rbyte = reader.read_unaligned(UnalignedRWMode(3)).unwrap() as u8;
            assert!(rbyte == byte,"{rbyte:8b} != {byte:8b}");
        }
    }
}
#[cfg(test)]
mod test_writter{
    use super::*;
    #[test]
    fn half_aligned(){
        let bytes:[u8;8] = [0x01,0x23,0x45,0x67,0x89,0xAB,0xCD,0xEF];
        let mut result = Vec::with_capacity(8);
        {
            let mut writter = UnalignedWriter::new(&mut result);
            for byte in 0..0x10{
                writter.write_unaligned(UnalignedRWMode(4),byte).unwrap();
            }
        }
        assert!(result == bytes,"{:x?} != {:x?}",result,bytes);
    }
    fn bin_3_aligned(){
        let bytes:[u8;3] = [0b00000101,0b00111001,0b01110111];
        let mut result = Vec::with_capacity(8);
        {
            let mut writter = UnalignedWriter::new(&mut result);
            for byte in 0..0x8{
                writter.write_unaligned(UnalignedRWMode(3),byte).unwrap();
            }
        }
        assert!(result == bytes,"{:x?} != {:x?}",result,bytes);
    }
    #[test]
    fn one_and_half_aligned(){
        let expected:[u8;12] = [0x00,0x0F,0xFF,0xF0,0xFA,0xBC,0x1A,0x58,0x54,0x48,0x55,0x64]; 
        let bytes:[u16;8] = [0x000,0xFFF,0xF0F,0xABC,0x1A5,0x854,0x485,0x564];
        let mut result = Vec::with_capacity(8);
        {
            let mut writter = UnalignedWriter::new(&mut result);
            for byte in bytes{
                writter.write_unaligned(UnalignedRWMode(12),byte as u64).unwrap();
            }
        }
        assert!(result == expected,"{:x?} != {:x?}",result,expected);
    }
}
