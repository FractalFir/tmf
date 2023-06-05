use std::io::{BufWriter, Read, Result, Write};
#[cfg(not(feature = "byte_rw"))]
type UnalignedStorage = usize;
#[cfg(feature = "byte_rw")]
type UnalignedStorage = usize;

const UNALIGNED_STORAGE_BITS: u8 = (std::mem::size_of::<UnalignedStorage>() * 8) as u8;
pub struct UnalignedReader<R: Read> {
    /// Buff Reader used to speedup reads in some cases.
    reader: R,
    /// current byte read from file.
    current_byte: UnalignedStorage,
    /// Amount of bits that have been already read.
    bits_read: u8,
}
impl<R: Read> UnalignedReader<R> {
    fn read_to_internal_storage(&mut self) -> Result<()> {
        // For u8, use simpler, old version. For others, this branch can never be taken an will be optimised out.
        if std::mem::size_of::<UnalignedStorage>() == 1 {
            let mut tmp: [u8; std::mem::size_of::<UnalignedStorage>()] =
                [0; std::mem::size_of::<UnalignedStorage>()];
            self.bits_read = 0;
            self.reader.read_exact(&mut tmp)?;
            self.current_byte = tmp[0] as UnalignedStorage;
        } else {
            let mut tmp: [u8; std::mem::size_of::<UnalignedStorage>()] =
                [0; std::mem::size_of::<UnalignedStorage>()];
            let bits_read = self.reader.read(&mut tmp)? * 8;
            if bits_read == 0 {
                return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, ""));
            }
            self.bits_read = (8 * std::mem::size_of::<UnalignedStorage>() - bits_read) as u8;
            self.current_byte = UnalignedStorage::from_be_bytes(tmp);
        }
        Ok(())
    }
    /// Reads exactly one bit from UBA.
    pub fn read_bit(&mut self) -> Result<bool> {
        // If all bits in current_byte read, read new byte with new bits, and set amount of bits bits_read in current bit back to 0.
        if self.bits_read >= UNALIGNED_STORAGE_BITS {
            self.read_to_internal_storage()?;
            if (UNALIGNED_STORAGE_BITS - self.bits_read) < 1 {
                use std::io::{Error, ErrorKind};
                return Err(Error::from(ErrorKind::UnexpectedEof));
            }
        }
        const BIT_MASK: UnalignedStorage = 1 << (UNALIGNED_STORAGE_BITS - 1);
        let res = self.current_byte & BIT_MASK;
        self.current_byte <<= 1;
        self.bits_read += 1;
        Ok(res != 0)
    }
    pub fn read2_unaligned(&mut self, mode: UnalignedRWMode) -> Result<(u64, u64)> {
        assert_ne!(mode.0, 0);
        if mode.0 > (u64::BITS / 2) as u8 {
            Ok((self.read_unaligned(mode)?, self.read_unaligned(mode)?))
        } else {
            let read_size = UnalignedRWMode(mode.0 * 2);
            let data = self.read_unaligned(read_size)?;
            let r1 = data >> mode.0;
            let r2 = data << (u64::BITS as u8 - mode.0) >> (u64::BITS as u8 - mode.0);
            Ok((r1, r2))
        }
    }
    pub fn read_pair_unaligned(
        &mut self,
        mode1: UnalignedRWMode,
        mode2: UnalignedRWMode,
    ) -> Result<(u64, u64)> {
        if mode1.0 + mode2.0 > u64::BITS as u8 {
            Ok((self.read_unaligned(mode1)?, self.read_unaligned(mode2)?))
        } else {
            let read_size = UnalignedRWMode(mode1.0 + mode2.0);
            let data = self.read_unaligned(read_size)?;
            let r1 = data >> mode1.0;
            let r2 = data << (u64::BITS as u8 - mode1.0) >> (u64::BITS as u8 - mode1.0);
            Ok((r1, r2))
        }
    }
    pub fn read3_unaligned(&mut self, mode: UnalignedRWMode) -> Result<(u64, u64, u64)> {
        if mode.0 > (u64::BITS / 3) as u8 {
            let res_1_2 = self.read2_unaligned(mode)?;
            Ok((res_1_2.0, res_1_2.1, self.read_unaligned(mode)?))
        } else {
            let read_size = UnalignedRWMode(mode.0 * 3);
            let data = self.read_unaligned(read_size)?;
            let r1 = data >> ((mode.0) * 2);
            let r_23 = data << (u64::BITS as u8 - mode.0 * 2) >> (u64::BITS as u8 - mode.0 * 2);
            let r2 = r_23 >> mode.0;
            let r3 = r_23 << (u64::BITS as u8 - mode.0) >> (u64::BITS as u8 - mode.0);
            Ok((r1, r2, r3))
        }
    }
    /// Reads *mode.0* bits from self, keeping internal alignment
    pub fn read_unaligned(&mut self, mode: UnalignedRWMode) -> Result<u64> {
        if mode.0 == 0 || mode.0 >= u64::BITS as u8 {
            return Ok(0);
        };
        // Prepare result integer, in which read result is stored.
        let mut res: u64 = 0;
        // Total bits remaining to read
        let mut total_read = mode.0;
        while total_read != 0 {
            // If all bits in current_byte read, read new byte with new bits, and set amount of bits bits_read in current bit back to 0.
            if self.bits_read >= UNALIGNED_STORAGE_BITS {
                self.read_to_internal_storage()?;
            }
            // Get amount of bits to read in current iteration: either all bits left in current_byte, or all bits remaining to read, whichever lower
            let read_ammount = total_read.min(UNALIGNED_STORAGE_BITS - self.bits_read);
            // Move res by amount of bits bits_read in current iteration to prepare res for reading into in next iteration.
            res <<= read_ammount;
            // Calculate offset of bits_read bits in current byte.
            let read_offset = UNALIGNED_STORAGE_BITS - read_ammount;
            // Read bits in current_byte at read_offset into res.
            res |= (self.current_byte >> read_offset) as u64;
            // Increment amount of bits already bits_read.
            self.bits_read += read_ammount;
            // If read less than whole current byte, move current byte in such a way that next bits to read are the leftmost bits.
            if read_ammount < UNALIGNED_STORAGE_BITS {
                self.current_byte <<= read_ammount;
            }
            // Decrement total amount of bits left to read
            total_read -= read_ammount;
        }
        Ok(res)
    }
    /// Creates new Unaligned Reader form *r*
    pub fn new(r: R) -> Self {
        let reader = r;
        let current_byte = 0; //read this
        let bits_read = UNALIGNED_STORAGE_BITS;
        Self {
            current_byte,
            bits_read,
            reader,
        }
    }
}
pub struct UnalignedWriter<W: Write> {
    /// Buffered writer used to speedup writes in some cases.
    writer: BufWriter<W>,
    /// The next byte
    next_byte: u8,
    /// Amount of bit already written in this byte.
    written: u8,
}
impl<W: Write> Drop for UnalignedWriter<W> {
    fn drop(&mut self) {
        if self.written > 0 {
            self.writer
                .write_all(&[self.next_byte])
                .expect("Unaligned writer failed to write last byte when dropped")
        };
        self.writer
            .flush()
            .expect("Unaligned writer failed to flush when dropped");
    }
}
impl<W: Write> UnalignedWriter<W> {
    pub fn new(w: W) -> Self {
        let writer = BufWriter::new(w);
        let next_byte = 0;
        let written = 0;
        Self {
            next_byte,
            written,
            writer,
        }
    }
    /// Writes exactly one bit from UBA.
    pub fn write_bit(&mut self, bit: bool) -> Result<()> {
        const BIT_PREC: UnalignedRWMode = UnalignedRWMode::precision_bits(1);
        self.write_unaligned(BIT_PREC, bit as u64)
    }
    #[inline(always)]
    pub fn write_unaligned(&mut self, mode: UnalignedRWMode, mut data: u64) -> Result<()> {
        if mode.0 == 0 {
            return Ok(());
        };
        let mut total_write = mode.0;
        // Move all the bits to write to left, so the first bit to write is the leftmost bit.
        data <<= 64 - total_write;

        while total_write > 0 {
            // Calculate how many bits to write in current iteration: either how many bits unwritten are left in next byte(bits in byte - nits written) or all the bits remaining in data, if they fit.
            let curr_write = total_write.min(8 - self.written);
            // Take *curr_write* bits of data
            let curr_bits = {
                //Get *curr_write* leftmostmost bits of data
                let bits = (data >> (64 - curr_write)) as u8;
                //Move data to left by *curr_write* ensuring the next bits to read are the rightmost bits(this assumption is used to extract bits to write).
                data <<= curr_write;
                bits
            };
            // Calculate the offset within vurrent byte at which to put all data(byte size - already occupied bits - amount of bits to write
            let bit_offset = 8 - self.written - curr_write;
            // Move the current bits by the bit offset to but them in right place to write to next_byte
            let curr_bits = curr_bits << bit_offset;
            // Or the current bits and next_byte to write to it
            self.next_byte |= curr_bits;
            // Increment amount of bits written in next byte
            self.written += curr_write;
            // Decrease amount of bits left to write by amount of bits written in current iteration
            total_write -= curr_write;
            // If full byte written, write(flush) it to output, and reset other parameters to prepare for next writes
            if self.written >= 8 {
                self.writer.write_all(&[self.next_byte])?;
                self.written = 0;
                self.next_byte = 0;
            }
        }
        Ok(())
    }
    /// This function should be called when and *ONLY* when writer is no longer in use. It flushes the writer an allows user to handle errors.
    pub fn flush(&mut self) -> Result<()> {
        self.writer.flush()
    }
}
#[derive(Clone, Copy)]
pub struct UnalignedRWMode(u8);
impl UnalignedRWMode {
    pub const fn precision_bits(bits: u8) -> Self {
        Self(bits)
    }
    pub const fn bits(&self) -> u8 {
        self.0
    }
}
#[cfg(test)]
const CHANIGING_ALGHMENT_EXPECTED: [u8; 252] = [
    0b11001101, 0b00001010, 0b00110000, 0b01110000, 0b10000000, 0b01001000, 0b00010100, 0b00000010,
    0b11000000, 0b00110000, 0b00000001, 0b10100000, 0b00000111, 0b00000000, 0b00001111, 0b00000000,
    0b00010000, 0b00000000, 0b00001000, 0b10000000, 0b00000010, 0b01000000, 0b00000000, 0b01001100,
    0b00000000, 0b00000101, 0b00000000, 0b00000000, 0b00101010, 0b00000000, 0b00000000, 0b10110000,
    0b00000000, 0b00000001, 0b01110000, 0b00000000, 0b00000001, 0b10000000, 0b00000000, 0b00000000,
    0b11001000, 0b00000000, 0b00000000, 0b00110100, 0b00000000, 0b00000000, 0b00000110, 0b11000000,
    0b00000000, 0b00000000, 0b01110000, 0b00000000, 0b00000000, 0b00000011, 0b10100000, 0b00000000,
    0b00000000, 0b00001111, 0b00000000, 0b00000000, 0b00000000, 0b00011111, 0b00000000, 0b00000000,
    0b00000000, 0b00100000, 0b00000000, 0b00000000, 0b00000000, 0b00010000, 0b10000000, 0b00000000,
    0b00000000, 0b00000100, 0b01000000, 0b00000000, 0b00000000, 0b00000000, 0b10001100, 0b00000000,
    0b00000000, 0b00000000, 0b00001001, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b01001010,
    0b00000000, 0b00000000, 0b00000000, 0b00000001, 0b00110000, 0b00000000, 0b00000000, 0b00000000,
    0b00000010, 0b01110000, 0b00000000, 0b00000000, 0b00000000, 0b00000010, 0b10000000, 0b00000000,
    0b00000000, 0b00000000, 0b00000001, 0b01001000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
    0b01010100, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00001010, 0b11000000, 0b00000000,
    0b00000000, 0b00000000, 0b00000000, 0b10110000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
    0b00000101, 0b10100000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00010111, 0b00000000,
    0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00101111, 0b00000000, 0b00000000, 0b00000000,
    0b00000000, 0b00000000, 0b00110000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
    0b00011000, 0b10000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000110, 0b01000000,
    0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b11001100, 0b00000000, 0b00000000,
    0b00000000, 0b00000000, 0b00000000, 0b00001101, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
    0b00000000, 0b00000000, 0b01101010, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
    0b00000001, 0b10110000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000011,
    0b01110000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000011, 0b10000000,
    0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000001, 0b11001000, 0b00000000,
    0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b01110100, 0b00000000, 0b00000000,
    0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00001110, 0b11000000, 0b00000000, 0b00000000,
    0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b11110000, 0b00000000, 0b00000000, 0b00000000,
    0b00000000, 0b00000000, 0b00000000, 0b00000111, 0b10100000, 0b00000000, 0b00000000, 0b00000000,
    0b00000000, 0b00000000, 0b00000000, 0b00011111, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
    0b00000000, 0b00000000, 0b00000000, 0b00111111,
];
#[cfg(test)]
mod test_reader {
    use super::*;
    #[test]
    fn half_aligned() {
        let bytes: [u8; 8] = [0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF];
        let mut reader = UnalignedReader::new(&bytes as &[u8]);
        for byte in 0..0x10 {
            let rbyte = reader.read_unaligned(UnalignedRWMode(4)).unwrap() as u8;
            assert!(rbyte == byte, "{rbyte} != {byte}");
        }
    }
    #[test]
    fn read2_half_aligned() {
        let bytes: [u8; 8] = [0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF];
        let mut reader = UnalignedReader::new(&bytes as &[u8]);
        for byte in 0..(0x10 / 2) {
            let (r1, r2) = reader.read2_unaligned(UnalignedRWMode(4)).unwrap();
            assert_eq!(r1, byte * 2);
            assert_eq!(r2, byte * 2 + 1);
        }
    }
    #[test]
    fn read3_half_aligned() {
        let bytes: [u8; 8] = [0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF];
        let mut reader = UnalignedReader::new(&bytes as &[u8]);
        for byte in 0..(0x10 / 3) {
            let (r1, r2, r3) = reader.read3_unaligned(UnalignedRWMode(4)).unwrap();
            assert_eq!(r1, byte * 3, "r1");
            assert_eq!(r2, byte * 3 + 1, "r2");
            assert_eq!(r3, byte * 3 + 2, "r3");
        }
    }
    #[test]
    fn read_bit() {
        let bytes: [u8; 2] = [0b1110_0010, 0b1010_0101];
        let expected: [bool; 16] = [
            true, true, true, false, false, false, true, false, true, false, true, false, false,
            true, false, true,
        ];
        let mut reader = UnalignedReader::new(&bytes as &[u8]);
        for val in expected {
            let rval = reader.read_bit().unwrap();
            //println!("{val} {rval}");
            assert_eq!(val, rval);
        }
    }
    #[test]
    fn one_and_half_aligned() {
        let bytes: [u8; 12] = [
            0x00, 0x0F, 0xFF, 0xF0, 0xFA, 0xBC, 0x1A, 0x58, 0x54, 0x48, 0x55, 0x64,
        ];
        let expected: [u16; 8] = [0x000, 0xFFF, 0xF0F, 0xABC, 0x1A5, 0x854, 0x485, 0x564];
        let mut reader = UnalignedReader::new(&bytes as &[u8]);
        for byte in expected {
            let rbyte = reader.read_unaligned(UnalignedRWMode(12)).unwrap() as u16;
            assert!(rbyte == byte, "{rbyte} != {byte}");
        }
    }
    #[test]
    fn bin_3_aligned() {
        // 000 001 01|0 011 100 1|01 110 111
        let bytes: [u8; 3] = [0b00000101, 0b00111001, 0b01110111];
        let mut reader = UnalignedReader::new(&bytes as &[u8]);
        for byte in 0..8 {
            let rbyte = reader.read_unaligned(UnalignedRWMode(3)).unwrap() as u8;
            assert!(rbyte == byte, "{rbyte:8b} != {byte:8b}");
        }
    }
    #[test]
    fn changing_alignment() {
        let mut reader = UnalignedReader::new(&CHANIGING_ALGHMENT_EXPECTED as &[u8]);
        for byte in 1..64 {
            assert!(reader.read_unaligned(UnalignedRWMode(byte)).unwrap() == byte as u64);
        }
    }
}
#[cfg(test)]
mod test_writter {
    use super::*;
    #[test]
    fn half_aligned() {
        let bytes: [u8; 8] = [0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF];
        let mut result = Vec::with_capacity(8);
        {
            let mut writter = UnalignedWriter::new(&mut result);
            for byte in 0..0x10 {
                writter.write_unaligned(UnalignedRWMode(4), byte).unwrap();
            }
        }
        assert!(result == bytes, "{:x?} != {:x?}", result, bytes);
    }
    #[test]
    fn bin_3_aligned() {
        let bytes: [u8; 3] = [0b00000101, 0b00111001, 0b01110111];
        let mut result = Vec::with_capacity(8);
        {
            let mut writter = UnalignedWriter::new(&mut result);
            for byte in 0..0x8 {
                writter.write_unaligned(UnalignedRWMode(3), byte).unwrap();
            }
        }
        for i in 0..3 {
            assert!(
                result[i] == bytes[i],
                "{:08b} != {:08b}",
                result[i],
                bytes[i]
            );
        }
        assert!(result == bytes, "{:x?} != {:x?}", result, bytes);
    }
    #[test]
    fn one_and_half_aligned() {
        let expected: [u8; 12] = [
            0x00, 0x0F, 0xFF, 0xF0, 0xFA, 0xBC, 0x1A, 0x58, 0x54, 0x48, 0x55, 0x64,
        ];
        let bytes: [u16; 8] = [0x000, 0xFFF, 0xF0F, 0xABC, 0x1A5, 0x854, 0x485, 0x564];
        let mut result = Vec::with_capacity(8);
        {
            let mut writter = UnalignedWriter::new(&mut result);
            for byte in bytes {
                writter
                    .write_unaligned(UnalignedRWMode(12), byte as u64)
                    .unwrap();
            }
        }
        assert!(result == expected, "{:x?} != {:x?}", result, expected);
    }
    #[test]
    fn changing_alignment() {
        let mut result = Vec::with_capacity(8);
        {
            let mut writter = UnalignedWriter::new(&mut result);
            for byte in 1..64 {
                writter
                    .write_unaligned(UnalignedRWMode(byte), byte as u64)
                    .unwrap();
            }
        }
        assert!(result == CHANIGING_ALGHMENT_EXPECTED);
    }
}
