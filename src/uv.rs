use crate::unaligned_rw::{UnalignedWriter,UnalignedRWMode};
use std::io::{Write,Result};
pub fn save_uvs<W:Write>(uvs:&[(f32,f32)],writer:&mut W,precision:f32)->Result<()>{
    let precision = (1.0/precision).log2().ceil() as u8;
    let multpiler = ((1<<precision) - 1) as f32;
    let precision = UnalignedRWMode::precision_bits(precision);
    let mut writer = UnalignedWriter::new(writer);
    for uv in uvs.iter(){
        let x = (uv.0*multpiler) as u64;
        let y = (uv.1*multpiler) as u64;
        writer.write_unaligned(precision,x)?;
        writer.write_unaligned(precision,y)?;
    }
    writer.flush()?;
    Ok(())
}
