#[derive(Clone)]
pub struct MaterialInfo {
    /// Custom field that can be used to store info about all materials in object, named that way only because it is more or less analogous to .obj material library.
    material_lib: String,
    // Names of all materials in model
    materials: Box<[String]>,
}
