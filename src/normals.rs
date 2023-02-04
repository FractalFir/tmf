use crate::unaligned_rw::{UnalignedRWMode, UnalignedReader, UnalignedWriter};
use crate::{FloatType, IndexType, Vector3};
use std::io::{Read, Result, Write};
#[derive(Clone, Copy)]
pub struct NormalPrecisionMode(u8);
impl NormalPrecisionMode {
    pub fn from_deg_dev(deg: FloatType) -> Self {
        let prec = (90.0 / deg).log2().ceil() as u8;
        Self(prec)
    }
    pub fn bits(&self) -> u8 {
        self.0
    }
}
const SIGN_PREC: UnalignedRWMode = UnalignedRWMode::precision_bits(1);
use crate::utilis::*;
pub fn normalize_arr(normals: &mut [Vector3]) {
    for normal in normals {
        *normal = normalize(*normal);
    }
}
fn normal_to_encoding(
    normal: Vector3,
    precision: &NormalPrecisionMode,
) -> (u64, u64, bool, bool, bool) {
    let multiplier = ((1 << precision.0) - 1) as FloatType;
    //Calculate asine
    let xy = (normal.0.abs(), normal.1.abs());
    let xy_mag = (xy.0 * xy.0 + xy.1 * xy.1).sqrt();
    let xy = (xy.0 / xy_mag, xy.1 / xy_mag);
    let asine = xy.0.asin();

    let asine = asine / (PI / 2.0);
    //
    let asine = (asine * multiplier) as u64;
    let z = (normal.2.abs() * multiplier) as u64;
    let sx = normal.0 < 0.0;
    let sy = normal.1 < 0.0;
    let sz = normal.2 < 0.0;
    (asine, z, sx, sy, sz)
}
fn normal_from_encoding(
    asine: u64,
    z: u64,
    sx: bool,
    sy: bool,
    sz: bool,
    precision: NormalPrecisionMode,
) -> Vector3 {
    let divisor = ((1 << precision.0) - 1) as FloatType;
    //Read raw asine
    let asine = (asine as FloatType) / divisor;
    //Convert asine form 0-1 to 0-tau
    let asine = asine * (PI / 2.0);
    //Read xyz component
    let z = (z as FloatType) / divisor;
    #[cfg(feature = "fast_trig")]
    let x = fsin(asine as fprec) as FloatType;
    #[cfg(not(feature = "fast_trig"))]
    let (x, y) = asine.sin_cos();
    #[cfg(feature = "fast_trig")]
    let y = (1.0 - x * x).sqrt();
    // Calculate XY magnitude
    let xy_mag = (1.0 - z * z).sqrt();
    // Adjust x an y
    let y = y * xy_mag;
    let x = x * xy_mag;
    // Set signs
    let x = if sx { -x } else { x };
    let y = if sy { -y } else { y };
    let z = if sz { -z } else { z };
    (x, y, z)
}
const PI: FloatType = std::f64::consts::PI as FloatType;
#[inline(always)]
fn save_normal<W: Write>(
    normal: Vector3,
    precision: NormalPrecisionMode,
    writer: &mut UnalignedWriter<W>,
) -> Result<()> {
    let (asine, z, sx, sy, sz) = normal_to_encoding(normal, &precision);
    let main_prec = UnalignedRWMode::precision_bits(precision.0);

    writer.write_unaligned(SIGN_PREC, sx as u64)?;
    writer.write_unaligned(SIGN_PREC, sy as u64)?;
    writer.write_unaligned(SIGN_PREC, sz as u64)?;
    writer.write_unaligned(main_prec, asine)?;
    writer.write_unaligned(main_prec, z)?;

    Ok(())
}
#[inline(always)]
fn read_normal<R: Read>(
    precision: NormalPrecisionMode,
    reader: &mut UnalignedReader<R>,
) -> Result<Vector3> {
    let main_prec = UnalignedRWMode::precision_bits(precision.0);
    // Get signs of x y z component
    let sx = reader.read_unaligned(SIGN_PREC)? != 0;
    let sy = reader.read_unaligned(SIGN_PREC)? != 0;
    let sz = reader.read_unaligned(SIGN_PREC)? != 0;
    let asine = reader.read_unaligned(main_prec)?;
    let z = reader.read_unaligned(main_prec)?;

    Ok(normal_from_encoding(asine, z, sx, sy, sz, precision))
}
pub(crate) fn save_normal_array<W: Write>(
    normals: &[Vector3],
    writer: &mut W,
    precision: NormalPrecisionMode,
) -> Result<()> {
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
pub(crate) fn read_normal_array<R: Read>(reader: &mut R) -> Result<Box<[Vector3]>> {
    let count = {
        let mut tmp = [0; std::mem::size_of::<u64>()];
        reader.read_exact(&mut tmp)?;
        u64::from_le_bytes(tmp)
    } as usize;
    let precision = NormalPrecisionMode({
        let mut tmp: [u8; 1] = [0; 1];
        reader.read_exact(&mut tmp)?;
        tmp[0]
    });
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
    pub const NORM_PREC_LOW: NormalPrecisionMode = NormalPrecisionMode(7);
    pub const NORM_PREC_MID: NormalPrecisionMode = NormalPrecisionMode(10);
    pub const NORM_PREC_HIGH: NormalPrecisionMode = NormalPrecisionMode(13);
    fn dot(a: Vector3, b: Vector3) -> FloatType {
        a.0 * b.0 + a.1 * b.1 + a.2 * b.2
    }
    fn test_save(normal: Vector3) {
        let mut res = Vec::with_capacity(64);
        let precision = NormalPrecisionMode(14);
        {
            let mut writter = UnalignedWriter::new(&mut res);
            save_normal(normal, precision, &mut writter).unwrap();
        }
        let mut reader = UnalignedReader::new(&res as &[u8]);
        let r_normal = read_normal(precision, &mut reader).unwrap();
        let n_dot = (1.0 - dot(r_normal, normal)) * 180.0;
        assert!(
            n_dot < 0.01,
            "expected:{normal:?} != read:{r_normal:?} angle:{n_dot}"
        );
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
        use rand::{thread_rng, Rng};
        let mut rng = thread_rng();
        for _ in 0..100000 {
            let norm = (
                rng.gen::<FloatType>() * 2.0 - 1.0,
                rng.gen::<FloatType>() * 2.0 - 1.0,
                rng.gen::<FloatType>() * 2.0 - 1.0,
            );
            let norm = normalize(norm);
            test_save(norm);
        }
    }
    #[test]
    fn rw_normal_array() {
        use rand::{thread_rng, Rng};
        let mut rng = thread_rng();
        let count = ((rng.gen::<IndexType>() % 0x800) + 0x800) as usize;
        let mut res = Vec::with_capacity(count);
        let mut normals = Vec::with_capacity(count);
        for _ in 0..count {
            let norm = (
                rng.gen::<FloatType>() * 2.0 - 1.0,
                rng.gen::<FloatType>() * 2.0 - 1.0,
                rng.gen::<FloatType>() * 2.0 - 1.0,
            );
            let norm = normalize(norm);
            normals.push(norm);
        }
        save_normal_array(&normals, &mut res, NORM_PREC_HIGH).unwrap();
        let r_normals = read_normal_array(&mut (&res as &[u8])).unwrap();
        for i in 0..count {
            let r_normal = r_normals[i];
            let normal = normals[i];
            let n_dot = (1.0 - dot(r_normal, normal)) * 180.0;
            assert!(
                n_dot < 0.1,
                "expected:{normal:?} != read:{r_normal:?} angle:{n_dot}"
            );
        }
    }
    #[test]
    #[cfg(feature = "fast_trig")]
    fn test_fast_sin() {
        for i in 1..100000 {
            let x: fprec = (100000.0 / (i as fprec)) * std::f64::consts::PI;
            let sin = x.sin();
            let fsin = fsin(x);
            let dt = sin - fsin;
            assert!(dt < 0.000333, "{x}:{sin} - {fsin} = {dt}");
        }
    }
}
// Calcuates size of the map necesary for map pruning of normals
fn map_size(prec: &NormalPrecisionMode) -> u128 {
    let asine_bits = prec.bits();
    let z_bits = prec.bits();
    let sign_bits = 1 * 3;
    let toatal_bits = asine_bits + z_bits + sign_bits;
    1 << toatal_bits
}
fn normal_to_map_index(normal: Vector3, prec: &NormalPrecisionMode) -> u64 {
    let (asine, z, sx, sy, sz) = normal_to_encoding(normal, prec);
    let (sx, sy, sz) = (sx as u64, sy as u64, sz as u64);
    let z_offset = prec.bits();
    let s_offset = z_offset + prec.bits();
    asine | (z << z_offset) | (sx << s_offset) | (sy << (s_offset + 1)) | (sz << (s_offset + 2))
}
use crate::TMFPrecisionInfo;
pub(crate) fn map_prune(
    normals: &mut Vec<Vector3>,
    normal_faces: &mut [IndexType],
    map_max: usize,
    prec: &TMFPrecisionInfo,
) {
    let map_size = map_size(&prec.normal_precision);
    // Map size exceeds maximal allowed map size, return.
    if map_size > map_max as u128 {
        return;
    }
    let map_size = map_size as usize;
    let mut map: Vec<IndexType> = vec![IndexType::MAX; map_size];
    // New, smaller normals
    let mut new_normals = Vec::with_capacity(normals.len());
    for normal in normals.iter() {
        //
        // Calculate index into the map
        let index = normal_to_map_index(*normal, &prec.normal_precision) as usize;
        if map[index] == IndexType::MAX {
            let new_map_index = new_normals.len();
            map[index] = new_map_index as IndexType;
            new_normals.push(*normal);
        }
    }
    for mut index in normal_faces {
        *index =
            map[normal_to_map_index(normals[*index as usize], &prec.normal_precision) as usize];
        //assert!(((*index) as usize) < new_normals.len());
    }
    *normals = new_normals;
}
