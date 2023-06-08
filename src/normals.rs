use crate::read_extension::ReadExt;
use crate::unaligned_rw::{UnalignedRWMode, UnalignedReader, UnalignedWriter};
use crate::{FloatType, TMFImportError, Vector3, MAX_SEG_SIZE};
#[cfg(not(feature = "double_precision"))]
use std::f32::consts::FRAC_PI_2;
#[cfg(feature = "double_precision")]
use std::f64::consts::FRAC_PI_2;
use std::io::{Read, Write};

#[derive(Clone, Copy, PartialEq)]
/// Setting dictating how much can any normal in a model deviate, expressed as an angle.
pub struct NormalPrecisionMode(u8);
impl NormalPrecisionMode {
    /// Creates [`NormalPrecisionMode`] from maximal allowed deviation angle in degrees, for radians use [`Self::from_rad_dev`]
    /// ```
    /// # use tmf::NormalPrecisionMode;
    /// // Maximal angle between compressed and original normal will be 1.0 degrees.
    /// let dev_1_deg = NormalPrecisionMode::from_deg_dev(1.0);
    /// // Maximal angle between compressed and original normal will be 0.01 degrees.
    /// let dev_0_point_01_deg = NormalPrecisionMode::from_deg_dev(0.01);
    /// // Maximal angle between compressed and original normal will be 5.0 degrees.
    /// let dev_5_deg = NormalPrecisionMode::from_deg_dev(5.0);
    /// ```
    pub fn from_deg_dev(deg: FloatType) -> Self {
        // TODO: use correct calculation here
        let prec = ((90.0 / deg).log2().ceil() as u8).max(1);
        Self(prec)
    }
    /// Creates NormalPrecisionMode from maximal allowed deviation angle in radians, for degrees use [`Self::from_deg_dev`]
    /// ```
    /// # use tmf::NormalPrecisionMode;
    /// // Maximal angle between compressed and original normal will be 0.01 radians
    /// let dev_0_point_01_rad = NormalPrecisionMode::from_rad_dev(0.01);
    /// // Maximal angle between compressed and original normal will be 0.05 radians
    /// let dev_0_point_05_rad = NormalPrecisionMode::from_rad_dev(0.05);
    /// ```
    pub fn from_rad_dev(rad: FloatType) -> Self {
        // TODO: use correct calculation here
        let prec = (FRAC_PI_2 / rad).log2().ceil() as u8;
        Self(prec)
    }
    pub(crate) fn bits(&self) -> u8 {
        self.0
    }
    pub(crate) fn from_bits(bits: u8) -> Self {
        Self(bits)
    }
}
impl Default for NormalPrecisionMode {
    /// Default precision of saved normals is 1.0 degrees
    ///```
    /// # use tmf::NormalPrecisionMode;
    /// let mode = NormalPrecisionMode::from_deg_dev(1.0);
    /// let default_mode = NormalPrecisionMode::default();
    /// // The same
    /// assert!(mode == default_mode);
    ///```
    fn default() -> Self {
        Self::from_deg_dev(1.0)
    }
}
//const SIGN_PREC: UnalignedRWMode = UnalignedRWMode::precision_bits(1);
use crate::utilis::*;
pub fn normalize_arr(normals: &mut [Vector3]) {
    for normal in normals {
        *normal = normalize(*normal);
    }
}
pub(crate) fn normal_to_encoding(normal: Vector3, precision: &NormalPrecisionMode) -> (u64, u64) {
    let multiplier = ((1 << precision.0) - 1) as FloatType;
    let norm = normal.0.abs() + normal.1.abs() + normal.2.abs();
    let mut nx = normal.0 / norm;
    let mut ny = normal.1 / norm;
    if !normal.2.is_sign_positive() {
        // fold over negative z
        (nx, ny) = ((1.0 - ny.abs()) * nx.signum(),
            (1.0 - nx.abs()) * ny.signum());
    }
    nx = (nx * 0.5 + 0.5) * multiplier;
    ny = (ny * 0.5 + 0.5) * multiplier;
    (nx as u64, ny as u64)
}
pub(crate) fn normal_from_encoding(a: u64, b: u64, precision: NormalPrecisionMode) -> Vector3 {
    let divisor = ((1_u64 << precision.0) - 1) as FloatType;
    let mut x = (a as FloatType) / divisor * 2.0 - 1.0;
    let mut y = (b as FloatType) / divisor * 2.0 - 1.0;
    let z = 1.0 - x.abs() - y.abs();
    x += x.signum() * z.min(0.0);
    y += y.signum() * z.min(0.0);
    normalize((x, y, z))
}
#[inline(always)]
fn save_normal<W: Write>(
    normal: Vector3,
    precision: NormalPrecisionMode,
    writer: &mut UnalignedWriter<W>,
) -> std::io::Result<()> {
    let (a, b) = normal_to_encoding(normal, &precision);
    let main_prec = UnalignedRWMode::precision_bits(precision.0);

    writer.write_unaligned(main_prec, a)?;
    writer.write_unaligned(main_prec, b)?;

    Ok(())
}
#[inline(always)]
fn read_normal<R: Read>(
    precision: NormalPrecisionMode,
    reader: &mut UnalignedReader<R>,
) -> std::io::Result<Vector3> {
    let main_prec = UnalignedRWMode::precision_bits(precision.0);
    let (a, b) = reader.read2_unaligned(main_prec)?;
    Ok(normal_from_encoding(a, b, precision))
}
pub(crate) fn save_normal_array<W: Write>(
    normals: &[Vector3],
    writer: &mut W,
    precision: NormalPrecisionMode,
) -> std::io::Result<()> {
    let count = (normals.len() as u64).to_le_bytes();
    writer.write_all(&count)?;
    writer.write_all(&[precision.0])?;
    let mut writer = UnalignedWriter::new(writer);
    for normal in normals {
        save_normal(*normal, precision, &mut writer)?;
    }
    writer.flush()?;
    Ok(())
}
pub(crate) fn read_normal_array<R: Read>(reader: &mut R) -> Result<Box<[Vector3]>, TMFImportError> {
    let count = reader.read_u64()? as usize;
    if count > MAX_SEG_SIZE {
        return Err(TMFImportError::SegmentTooLong);
    }
    let precision = reader.read_u8()?;
    if precision == 0 {}
    if precision >= u64::BITS as u8 {
        return Err(TMFImportError::InvalidPrecision(precision));
    }
    let precision = NormalPrecisionMode(precision);
    let mut reader = UnalignedReader::new(reader);
    let mut normals = Vec::with_capacity(count);
    for _ in 0..count {
        let normal = read_normal(precision, &mut reader)?;
        normals.push(normal);
    }
    Ok(normals.into())
}
#[cfg(test)]
mod test_normal {
    use super::*;
    const NORM_PREC_HIGH: NormalPrecisionMode = NormalPrecisionMode(13);
    const PI: FloatType = std::f64::consts::PI as FloatType;
    fn dot(a: Vector3, b: Vector3) -> FloatType {
        a.0 * b.0 + a.1 * b.1 + a.2 * b.2
    }
    /// Angle between two vectors in degrees.
    fn ang_between(a: Vector3, b: Vector3) -> FloatType {
        dot(a, b).clamp(0.0, 1.0).acos() * 180.0 / PI
    }
    fn test_save(normal: Vector3) {
        let mut res = Vec::with_capacity(64);
        let precision = NormalPrecisionMode::from_deg_dev(0.01);
        {
            let mut writer = UnalignedWriter::new(&mut res);
            save_normal(normal, precision, &mut writer).unwrap();
        }
        let mut reader = UnalignedReader::new(&res as &[u8]);
        let r_normal = read_normal(precision, &mut reader).unwrap();
        let ang_diff = ang_between(r_normal, normal);
        assert!(
            ang_diff < 0.01,
            "expected:{normal:?} != read:{r_normal:?} angle:{ang_diff}"
        );
    }
    /// Generator function for random normals (normalized).
    fn random_normals() -> impl Iterator<Item = Vector3> {
        use rand::{thread_rng, Rng};
        let mut rng = thread_rng();
        std::iter::from_fn(move || {
            Some((
                rng.gen::<FloatType>() * 2.0 - 1.0,
                rng.gen::<FloatType>() * 2.0 - 1.0,
                rng.gen::<FloatType>() * 2.0 - 1.0,
            ))
        })
        .map(normalize)
    }
    #[test]
    fn x_axis_rw() {
        test_save((1.0, 0.0, 0.0));
        test_save((-1.0, 0.0, 0.0));
    }
    #[test]
    fn y_axis_rw() {
        test_save((0.0, 1.0, 0.0));
        test_save((0.0, -1.0, 0.0));
    }
    #[test]
    fn z_axis_rw() {
        test_save((0.0, 0.0, 1.0));
        test_save((0.0, 0.0, -1.0));
    }
    #[test]
    fn random_axis_rw() {
        for norm in random_normals().take(100_000) {
            test_save(norm);
        }
    }
    #[test]
    fn rw_normal_array() {
        use rand::{thread_rng, Rng};
        let mut rng = thread_rng();
        let count = ((rng.gen::<crate::IndexType>() % 0x800) + 0x800) as usize;
        let mut res = Vec::with_capacity(count);
        let normals: Vec<_> = random_normals().take(count).collect();
        save_normal_array(&normals, &mut res, NORM_PREC_HIGH).unwrap();
        let r_normals = read_normal_array(&mut (&res as &[u8])).unwrap();
        for i in 0..count {
            let r_normal = r_normals[i];
            let normal = normals[i];
            let ang_diff = ang_between(r_normal, normal);
            assert!(
                ang_diff < 0.1,
                "expected:{normal:?} != read:{r_normal:?} angle:{ang_diff}"
            );
        }
    }
    #[test]
    #[cfg(feature = "fast_trig")]
    fn test_fast_sin() {
        for i in 1..100_000 {
            let x: fprec = (100000.0 / (i as fprec)) * std::f64::consts::PI;
            let sin = x.sin();
            let fsin = fsin(x);
            let dt = sin - fsin;
            assert!(dt < 0.000333, "{x}:{sin} - {fsin} = {dt}");
        }
    }
}
