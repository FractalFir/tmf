use crate::IndexType;
#[derive(Clone)]
pub struct MaterialInfo {
    // Names of all materials in model
    materials: Box<[String]>,
    material_groups: Box<[(IndexType, IndexType)]>,
}
impl MaterialInfo {
    fn create<M: Into<Box<[String]>>, MB: Into<Box<[(IndexType, IndexType)]>>>(
        materials: M,
        material_groups: MB,
    ) -> Self {
        Self {
            materials: materials.into(),
            material_groups: material_groups.into(),
        }
    }
}
