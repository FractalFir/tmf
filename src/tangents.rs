use crate::read_extension::ReadExt;
use crate::unaligned_rw::{UnalignedRWMode, UnalignedReader, UnalignedWriter};
use crate::FloatType;
use crate::NormalPrecisionMode;
/// A value describing handedness of tangent.
pub type HandednessType = FloatType;
#[derive(Clone, Copy)]
/// A value specifying how precise saved (x,y,z) values must be.
pub struct TangentPrecisionMode(crate::NormalPrecisionMode);
impl TangentPrecisionMode {
    fn normal_precision(&self) -> crate::NormalPrecisionMode {
        self.0
    }
    pub(crate) fn from_bits(bits: u8) -> Self {
        Self(NormalPrecisionMode::from_bits(bits))
    }
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
fn ht_to_bool(ht: HandednessType) -> bool {
    ht.is_sign_negative()
}
fn ht_from_bool(src: bool) -> HandednessType {
    if src {
        -1.0
    } else {
        1.0
    }
}
/// A representation of a Tangent.
pub type Tangent = (crate::Vector3, HandednessType);
fn tangent_to_encoding(
    tangent: Tangent,
    prec: TangentPrecisionMode,
) -> (u64, u64, bool, bool, bool, bool) {
    let normal = crate::normals::normal_to_encoding(tangent.0, &prec.normal_precision());
    let handeness = ht_to_bool(tangent.1);
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
    let handeness = ht_from_bool(handenes);
    (normal, handeness)
}
pub(crate) fn save_tangents<W: std::io::Write>(
    tangents: &[Tangent],
    prec: TangentPrecisionMode,
    target: &mut W,
) -> std::io::Result<()> {
    let count = (tangents.len() as u64).to_le_bytes();
    target.write_all(&count)?;
    let bits_prec = prec.normal_precision().bits();
    target.write_all(&[bits_prec])?;
    let mut writer = UnalignedWriter::new(target);
    let bits_prec = UnalignedRWMode::precision_bits(bits_prec);
    for tangent in tangents {
        let (asine, z, sx, sy, sz, handeness) = tangent_to_encoding(*tangent, prec);
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
pub(crate) fn read_tangents<R: std::io::Read>(src: &mut R) -> std::io::Result<Box<[Tangent]>> {
    let count = src.read_u64()?;
    let bits_prec = src.read_u8()?;
    let mut reader = UnalignedReader::new(src);
    let prec = UnalignedRWMode::precision_bits(bits_prec);
    let tan_prec = TangentPrecisionMode::from_bits(bits_prec);
    let mut tangents = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let handeness = reader.read_bit()?;
        let sx = reader.read_bit()?;
        let sy = reader.read_bit()?;
        let sz = reader.read_bit()?;
        let asine = reader.read_unaligned(prec)?;
        let z = reader.read_unaligned(prec)?;
        tangents.push(tangent_from_encoding(
            asine, z, sx, sy, sz, handeness, tan_prec,
        ));
    }
    Ok(tangents.into())
}
#[cfg(test)]
fn test_tangent(tangent: Tangent, prec: TangentPrecisionMode) -> FloatType {
    let encoded = tangent_to_encoding(tangent, prec);
    let decoded = tangent_from_encoding(
        encoded.0, encoded.1, encoded.2, encoded.3, encoded.4, encoded.5, prec,
    );
    assert_eq!(ht_to_bool(tangent.1), ht_to_bool(decoded.1));
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
    let handeness = ht_from_bool(rng.gen::<bool>());
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
#[cfg(test)]
#[test]
fn tangents_rw() {
    let prec = TangentPrecisionMode::default();
    let src_tangents: Vec<_> = (0..100_000).into_iter().map(|_| rand_tangent()).collect();
    let mut data = Vec::with_capacity(100_000);
    save_tangents(&src_tangents, prec, &mut data).unwrap();
    let target_tangents = read_tangents(&mut (&data as &[u8])).unwrap();
    for tangent in target_tangents.into_iter() {
        let degree = test_tangent(*tangent, prec);
        assert!(degree < 5.0);
    }
}
#[cfg(test)]
#[test]
#[cfg(feature = "obj_import")]
fn rw_susan_tmf_tans() {
    use crate::{TMFMesh, TMFPrecisionInfo};
    init_test_env();
    let mut file = std::fs::File::open("testing/susan.obj").unwrap();
    let (mut tmf_mesh, name) = TMFMesh::read_from_obj_one(&mut file).unwrap();
    tmf_mesh.verify().unwrap();
    let len = tmf_mesh.get_vertices().unwrap().len();
    tmf_mesh.set_tangents(
        (0..len)
            .into_iter()
            .map(|_| rand_tangent())
            .collect::<Vec<_>>(),
    );
    assert!(name == "Suzanne", "Name should be Suzanne but is {name}");
    let prec = TMFPrecisionInfo::default();
    let mut out = Vec::new();
    {
        tmf_mesh.write_tmf_one(&mut out, &prec, name).unwrap();
    }
    let (r_mesh, name) = TMFMesh::read_tmf_one(&mut (&out as &[u8])).unwrap();
    assert!(name == "Suzanne", "Name should be Suzanne but is {name}");
    r_mesh.verify().unwrap();
    r_mesh.get_tangents().unwrap();
}
#[cfg(test)]
#[test]
#[cfg(feature = "obj_import")]
fn rw_susan_tmf_tan_tris() {
    use crate::{TMFMesh, TMFPrecisionInfo};
    init_test_env();
    let mut file = std::fs::File::open("testing/susan.obj").unwrap();
    let (mut tmf_mesh, name) = TMFMesh::read_from_obj_one(&mut file).unwrap();
    tmf_mesh.verify().unwrap();
    let len = tmf_mesh.get_vertex_triangles().unwrap().len();
    tmf_mesh.set_tangent_triangles(
        (0..(len as crate::IndexType))
            .into_iter()
            .collect::<Vec<_>>(),
    );
    assert!(name == "Suzanne", "Name should be Suzanne but is {name}");
    let prec = TMFPrecisionInfo::default();
    let mut out = Vec::new();
    {
        tmf_mesh.write_tmf_one(&mut out, &prec, name).unwrap();
    }
    let (r_mesh, name) = TMFMesh::read_tmf_one(&mut (&out as &[u8])).unwrap();
    assert!(name == "Suzanne", "Name should be Suzanne but is {name}");
    r_mesh.verify().unwrap();
    r_mesh.get_tangent_triangles().unwrap();
}
#[cfg(test)]
fn init_test_env() {
    std::fs::create_dir_all("target/test_res").unwrap();
}
