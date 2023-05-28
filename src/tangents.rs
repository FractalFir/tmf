use crate::FloatType;
use crate::unaligned_rw::{UnalignedWriter,UnalignedReader,UnalignedRWMode};
use crate::NormalPrecisionMode;
#[derive(Clone, Copy, Debug)]
/// A value describing handedness of tangent.
pub struct HandenesType(FloatType);
#[derive(Clone, Copy)]
/// A value specifying how precise saved (x,y,z) values must be.
pub struct TangentPrecisionMode(crate::NormalPrecisionMode);
impl TangentPrecisionMode {
    fn normal_precision(&self) -> crate::NormalPrecisionMode {
        self.0
    }
    pub(crate) fn from_bits(bits:u8)->Self{Self(NormalPrecisionMode::from_bits(bits))}
}
impl TangentPrecisionMode {
    ///Creates a tangent precision mode with maximal deviation of (x,y,z) part being *deg* degrees.
    pub fn from_deg_dev(deg: FloatType) -> Self {
        Self(crate::NormalPrecisionMode::from_deg_dev(deg))
    }
    ///Creates a tangent precision mode with maximal deviation of (x,y,z) part being *rad* radians.
    pub fn from_rad_dev(rad: FloatType) -> Self {
        Self(crate::NormalPrecisionMode::from_rad_dev(rad))
    }
}
impl Default for TangentPrecisionMode {
    fn default() -> Self {
        Self::from_deg_dev(1.0)
    }
}
impl HandenesType {
    fn to_bool(self) -> bool {
        self.0.is_sign_negative()
    }
    fn from_bool(src: bool) -> Self {
        if src {
            Self(-1.0)
        } else {
            Self(1.0)
        }
    }
}
/// A representation of a Tangent.
pub type Tangent = (crate::Vector3, HandenesType);
fn tangent_to_encoding(
    tangent: Tangent,
    prec: TangentPrecisionMode,
) -> (u64, u64, bool, bool, bool, bool) {
    let normal = crate::normals::normal_to_encoding(tangent.0, &prec.normal_precision());
    let handeness = tangent.1.to_bool();
    (normal.0, normal.1, normal.2, normal.3, normal.4, handeness)
}
fn tangent_from_encoding(
    asine: u64,
    z: u64,
    sx: bool,
    sy: bool,
    sz: bool,
    handenes: bool,
    prec: TangentPrecisionMode,
) -> Tangent {
    let normal =
        crate::normals::normal_from_encoding(asine, z, sx, sy, sz, prec.normal_precision());
    let handeness = HandenesType::from_bool(handenes);
    (normal, handeness)
}
fn save_tangents<W:std::io::Write>(tangents:&[Tangent],prec: TangentPrecisionMode,target:&mut W)->std::io::Result<()>{
    let count = (tangents.len() as u64).to_le_bytes();
    target.write_all(&count)?;
    let bits_prec = prec.normal_precision().bits();
    target.write_all(&[bits_prec])?;
    let mut writer = UnalignedWriter::new(target);
    let bits_prec = UnalignedRWMode::precision_bits(bits_prec);
    for tangent in tangents{
        let (asine,z,sx,sy,sz,handeness) = tangent_to_encoding(*tangent,prec);
        writer.write_bit(handeness)?;
        writer.write_bit(sx)?;
        writer.write_bit(sy)?;
        writer.write_bit(sz)?;
        writer.write_unaligned(bits_prec, asine)?;
        writer.write_unaligned(bits_prec, z)?;
    }
    writer.flush()?;
    Ok(())
}
fn read_tangents<R:std::io::Read>(src:&mut R)->std::io::Result::<Box<[Tangent]>>{
    let count = {
        let mut tmp = [0;std::mem::size_of::<u64>()];
        src.read_exact(&mut tmp)?;
        u64::from_le_bytes(tmp)
    };
    let bits_prec = {
        let mut tmp = [0;std::mem::size_of::<u8>()];
        src.read_exact(&mut tmp)?;
        u8::from_le_bytes(tmp)
    };
    let mut reader = UnalignedReader::new(src);
    let prec = UnalignedRWMode::precision_bits(bits_prec);
    let tan_prec = TangentPrecisionMode::from_bits(bits_prec);
    let mut tangents = Vec::with_capacity(count as usize);
    for _ in 0..count{
        let handeness = reader.read_bit()?;
        let sx = reader.read_bit()?;
        let sy = reader.read_bit()?;
        let sz = reader.read_bit()?;
        let asine = reader.read_unaligned(prec)?;
        let z = reader.read_unaligned(prec)?;
        tangents.push(tangent_from_encoding(asine,z,sx,sy,sz,handeness,tan_prec));
    }
    Ok(tangents.into())
}
#[cfg(test)]
fn test_tangent(tangent: Tangent, prec: TangentPrecisionMode) -> FloatType {
    let encoded = tangent_to_encoding(tangent, prec);
    let decoded = tangent_from_encoding(
        encoded.0, encoded.1, encoded.2, encoded.3, encoded.4, encoded.5, prec,
    );
    assert_eq!(tangent.1.to_bool(), decoded.1.to_bool());
    (1.0 - crate::utilis::dot(decoded.0, tangent.0)) * 180.0
}
#[cfg(test)]
fn rand_tangent() -> Tangent {
    use rand::{thread_rng, Rng};
    let mut rng = thread_rng();
    let norm = (
        rng.gen::<FloatType>() - 0.5,
        rng.gen::<FloatType>() - 0.5,
        rng.gen::<FloatType>() - 0.5,
    );
    let norm = crate::utilis::normalize(norm);
    let handeness = HandenesType::from_bool(rng.gen::<bool>());
    (norm, handeness)
}
#[cfg(test)]
#[test]
fn tangent_rw() {
    let prec = TangentPrecisionMode::default();
    for _ in 0..100_000 {
        let tangent = rand_tangent();
        let degree = test_tangent(tangent, prec);
        assert!(degree < 5.0);
    }
}
