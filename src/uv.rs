use crate::unaligned_rw::{UnalignedRWMode, UnalignedReader, UnalignedWriter};
use crate::{FloatType, Vector2};
use std::io::{Read, Result, Write};
/// Setting dictating how precisely the UV coordinates should be saved.
#[derive(Clone, Copy, PartialEq)]
pub struct UvPrecisionMode(u8);
impl UvPrecisionMode {
    /// Creates a new [`UvPrecisionMode`] form texture resolution and maximal allowed deviation in pixels.
    /// ```
    /// # use tmf::UvPrecisionMode;
    /// let mode = UvPrecisionMode::form_texture_resolution(1024.0,0.1);
    /// ```
    pub fn form_texture_resolution(resolution: f32, pixel_dev: f32) -> Self {
        Self((resolution / pixel_dev).log2().ceil() as u8)
    }
}
impl Default for UvPrecisionMode {
    /// Default UV save precision. Assumes texture size 1024 and no more than .1 pixel deviation
    /// ```
    /// # use tmf::UvPrecisionMode;
    /// let mode = UvPrecisionMode::form_texture_resolution(1024.0,0.1);
    /// let default_mode = UvPrecisionMode::default();
    /// // The same
    /// assert!(mode == default_mode);
    /// ```
    fn default() -> Self {
        Self::form_texture_resolution(1024.0, 0.1)
    }
}
pub fn save_uvs<W: Write>(uvs: &[Vector2], writer: &mut W, precision: FloatType) -> Result<()> {
    let precision = (1.0 / precision).log2().ceil() as u8;
    let multpiler = ((1 << precision) - 1) as FloatType;
    writer.write_all(&[precision])?;
    writer.write_all(&(uvs.len() as u64).to_le_bytes())?;
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
pub fn read_uvs<R: Read>(reader: &mut R) -> Result<Box<[Vector2]>> {
    let precision = {
        let mut tmp = [0];
        reader.read_exact(&mut tmp)?;
        tmp[0]
    };
    let count = {
        let mut tmp = [0; std::mem::size_of::<u64>()];
        reader.read_exact(&mut tmp)?;
        u64::from_le_bytes(tmp)
    };
    let divisor = ((1 << precision) - 1) as FloatType;
    let precision = UnalignedRWMode::precision_bits(precision);
    let mut reader = UnalignedReader::new(reader);
    let mut uvs = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let x = (reader.read_unaligned(precision)? as FloatType) / divisor;
        let y = (reader.read_unaligned(precision)? as FloatType) / divisor;
        uvs.push((x, y));
    }
    Ok(uvs.into())
}
#[cfg(test)]
mod test {
    use super::*;
    use crate::IndexType;
    fn dst(a: Vector2, b: Vector2) -> FloatType {
        let dx = a.0 - b.0;
        let dy = a.1 - b.1;
        return (dx * dx + dy * dy).sqrt();
    }
    #[test]
    fn rw_uvs() {
        use rand::{thread_rng, Rng};
        let mut rng = thread_rng();
        let uv_count = (rng.gen::<IndexType>() % 0x800) + 0x800;
        let mut uvs = Vec::with_capacity(uv_count as usize);
        for _ in 0..uv_count {
            let x = rng.gen::<FloatType>();
            let y = rng.gen::<FloatType>();
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
