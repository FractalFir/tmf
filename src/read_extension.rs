pub(crate) trait ReadExt {
    fn read_u8(&mut self) -> std::io::Result<u8>;
    fn read_u16(&mut self) -> std::io::Result<u16>;
    fn read_u32(&mut self) -> std::io::Result<u32>;
    fn read_u64(&mut self) -> std::io::Result<u64>;

    fn read_f64(&mut self) -> std::io::Result<f64>;
}
impl<T: std::io::Read> ReadExt for T {
    fn read_u8(&mut self) -> std::io::Result<u8> {
        let mut tmp = [0u8];
        self.read_exact(&mut tmp)?;
        Ok(tmp[0])
    }
    fn read_u16(&mut self) -> std::io::Result<u16> {
        let mut tmp = [0; std::mem::size_of::<u16>()];
        self.read_exact(&mut tmp)?;
        Ok(u16::from_le_bytes(tmp))
    }
    fn read_u32(&mut self) -> std::io::Result<u32> {
        let mut tmp = [0; std::mem::size_of::<u32>()];
        self.read_exact(&mut tmp)?;
        Ok(u32::from_le_bytes(tmp))
    }
    fn read_u64(&mut self) -> std::io::Result<u64> {
        let mut tmp = [0; std::mem::size_of::<u64>()];
        self.read_exact(&mut tmp)?;
        Ok(u64::from_le_bytes(tmp))
    }

    fn read_f64(&mut self) -> std::io::Result<f64> {
        let mut tmp = [0; std::mem::size_of::<f64>()];
        self.read_exact(&mut tmp)?;
        Ok(f64::from_le_bytes(tmp))
    }
}
