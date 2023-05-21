use crate::FloatType;
#[derive(Clone, Copy, Debug)]
/// A value describing handedness of tangent.
pub struct HandenesType(bool);
#[derive(Clone, Copy)]
/// A value specifying how precise saved (x,y,z) values must be.
pub struct TangentPrecisionMode(crate::NormalPrecisionMode);
impl TangentPrecisionMode {
    fn normal_precision(&self) -> crate::NormalPrecisionMode {
        self.0
    }
}
impl TangentPrecisionMode{
    ///Creates a tangent precision mode with maximal deviation of (x,y,z) part being *deg* degrees. 
    fn from_deg_dev(deg: FloatType) -> Self{
        Self(crate::NormalPrecisionMode::from_deg_dev(deg))
    }
    ///Creates a tangent precision mode with maximal deviation of (x,y,z) part being *rad* radians. 
    fn from_rad_dev(rad: FloatType) -> Self{
        Self(crate::NormalPrecisionMode::from_rad_dev(rad))
    }
}
impl Default for TangentPrecisionMode{
    fn default()->Self{
        Self::from_deg_dev(1.0)
    }
}
impl HandenesType {
    fn to_bool(&self) -> bool {
        self.0
    }
    fn from_bool(src: bool) -> Self {
        Self(src)
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

