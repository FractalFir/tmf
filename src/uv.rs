use crate::unaligned_rw::{UnalignedRWMode, UnalignedReader, UnalignedWriter};
use std::io::{Read, Result, Write};
pub fn save_uvs<W: Write>(uvs: &[(f32, f32)], writer: &mut W, precision: f32) -> Result<()> {
    let precision = (1.0 / precision).log2().ceil() as u8;
    let multpiler = ((1 << precision) - 1) as f32;
    writer.write_all(&[precision])?;
    writer.write_all(&(uvs.len() as u32).to_le_bytes())?;
    let precision = UnalignedRWMode::precision_bits(precision);
    let mut writer = UnalignedWriter::new(writer);
    for uv in uvs.iter() {
        let x = (uv.0 * multpiler) as u64;
        let y = (uv.1 * multpiler) as u64;
        writer.write_unaligned(precision, x)?;
        writer.write_unaligned(precision, y)?;
    }
    writer.flush()?;
    Ok(())
}
pub fn read_uvs<R: Read>(reader: &mut R) -> Result<Box<[(f32, f32)]>> {
    let precision = {
        let mut tmp = [0];
        reader.read_exact(&mut tmp)?;
        tmp[0]
    };
    let count = {
        let mut tmp = [0; 4];
        reader.read_exact(&mut tmp)?;
        u32::from_le_bytes(tmp)
    };
    let divisor = ((1 << precision) - 1) as f32;
    let precision = UnalignedRWMode::precision_bits(precision);
    let mut reader = UnalignedReader::new(reader);
    let mut uvs = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let x = (reader.read_unaligned(precision)? as f32) / divisor;
        let y = (reader.read_unaligned(precision)? as f32) / divisor;
        uvs.push((x, y));
    }
    Ok(uvs.into())
}
#[cfg(test)]
mod test {
    use super::*;
    fn dst(a: (f32, f32), b: (f32, f32)) -> f32 {
        let dx = a.0 - b.0;
        let dy = a.1 - b.1;
        return (dx * dx + dy * dy).sqrt();
    }
    #[test]
    fn rw_uvs() {
        use rand::{thread_rng, Rng};
        let mut rng = thread_rng();
        let uv_count = (rng.gen::<u32>() % 0x800) + 0x800;
        let mut uvs = Vec::with_capacity(uv_count as usize);
        for _ in 0..uv_count {
            let x = rng.gen::<f32>();
            let y = rng.gen::<f32>();
            uvs.push((x, y));
        }
        let mut res = Vec::with_capacity(uv_count as usize);
        {
            save_uvs(&uvs, &mut res, 0.001).unwrap();
        }
        let r_uvs = read_uvs(&mut (&res as &[u8])).unwrap();
        assert!(r_uvs.len() == uvs.len());
        for i in 0..uvs.len() {
            assert!(
                dst(r_uvs[i], uvs[i]) < 0.01,
                "{:?} {:?} {}",
                r_uvs[i],
                uvs[i],
                dst(r_uvs[i], uvs[i])
            );
        }
    }
}
