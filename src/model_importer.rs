#[cfg(not(any(feature = "obj_import")))]
compile_error!("Feature \"model_importer\" is only useful when another feature using it is enabled(e.g. obj importer) and is otherwise useless dead code.");
use crate::{IndexType, TMFMesh, Vector2, Vector3};
const SMALL_VEC_CAP:usize = 8;
#[cfg(feature = "triangulation")]
include!("triangulation.rs");
pub(crate) struct ModelImporter {
    vertices: Vec<Vector3>,
    normals: Vec<Vector3>,
    uvs: Vec<Vector2>,
    vertex_triangles: Vec<IndexType>,
    normal_triangles: Vec<IndexType>,
    uv_triangles: Vec<IndexType>,
    name: String,
}
impl ModelImporter {
    pub(crate) fn new() -> Self {
        Self {
            vertices: Vec::new(),
            normals: Vec::new(),
            uvs: Vec::new(),
            vertex_triangles: Vec::new(),
            normal_triangles: Vec::new(),
            uv_triangles: Vec::new(),
            name: "".to_owned(),
        }
    }
    pub(crate) fn next_mesh(&mut self, mut name: String) -> Option<(TMFMesh, String)> {
        std::mem::swap(&mut self.name, &mut name);

        if self.vertex_triangles.is_empty() {
            return None;
        }
        let mut mesh = TMFMesh::empty();
        let (mut vertices, mut vertex_triangles) =
            (self.vertices.clone(), self.vertex_triangles.clone());
        let (mut normals, mut normal_triangles) =
            (self.normals.clone(), self.normal_triangles.clone());
        let (mut uvs, mut uv_triangles) = (self.uvs.clone(), self.uv_triangles.clone());

        crate::utilis::fast_prune(&mut vertices, &mut vertex_triangles);
        crate::utilis::fast_prune(&mut normals, &mut normal_triangles);
        crate::utilis::fast_prune(&mut uvs, &mut uv_triangles);

        mesh.set_vertices(vertices);
        mesh.set_normals(normals);
        mesh.set_uvs(uvs);

        mesh.set_vertex_triangles(vertex_triangles);
        mesh.set_normal_triangles(normal_triangles);
        mesh.set_uv_triangles(uv_triangles);

        self.vertex_triangles.clear();
        self.normal_triangles.clear();
        self.uv_triangles.clear();
        
        Some((mesh, name))
    }
    pub(crate) fn finish(self) -> (TMFMesh, String) {
        let mut mesh = TMFMesh::empty();

        let (mut vertices, mut vertex_triangles) =
            (self.vertices.clone(), self.vertex_triangles.clone());
        let (mut normals, mut normal_triangles) =
            (self.normals.clone(), self.normal_triangles.clone());
        let (mut uvs, mut uv_triangles) = (self.uvs.clone(), self.uv_triangles.clone());

        crate::utilis::fast_prune(&mut vertices, &mut vertex_triangles);
        crate::utilis::fast_prune(&mut normals, &mut normal_triangles);
        crate::utilis::fast_prune(&mut uvs, &mut uv_triangles);

        mesh.set_vertices(vertices);
        mesh.set_normals(normals);
        mesh.set_uvs(uvs);

        mesh.set_vertex_triangles(vertex_triangles);
        mesh.set_normal_triangles(normal_triangles);
        mesh.set_uv_triangles(uv_triangles);

        (mesh, self.name)
    }
    pub(crate) fn push_vertex(&mut self, vertex: Vector3) {
        self.vertices.push(vertex);
    }
    pub(crate) fn push_normal(&mut self, normal: Vector3) {
        self.normals.push(normal);
    }
    pub(crate) fn push_uv(&mut self, uv: Vector2) {
        self.uvs.push(uv);
    }
    // Pushes raw index into vertex triangles. WARNING: should be used with caution, because if number of pushed indices is not divisible by 3 this will corrupt the mesh
    fn push_vertex_index(&mut self, index: IndexType) {
        self.vertex_triangles.push(index);
    }
    // Pushes raw index into normal triangles. WARNING: should be used with caution, because if number of pushed indices is not divisible by 3 this will corrupt the mesh
    fn push_normal_index(&mut self, index: IndexType) {
        self.normal_triangles.push(index);
    }
    // Pushes raw index into uv triangles. WARNING: should be used with caution, because if number of pushed indices is not divisible by 3 this will corrupt the mesh
    fn push_uv_index(&mut self, index: IndexType) {
        self.uv_triangles.push(index);
    }
    pub(crate) fn push_face(
        &mut self,
        vertex_indices: &[IndexType],
        uv_indices: &[IndexType],
        normal_indices: &[IndexType],
    ) -> Result<(), String> {
        if !(vertex_indices.len() == uv_indices.len()
            && vertex_indices.len() == normal_indices.len())
        {
            return Err(format!("Number of face indices must be equal for each index type, but was: {} vertices, {} uvs, {} normals.",vertex_indices.len(),uv_indices.len(),normal_indices.len()));
        }
        match vertex_indices.len() {
            0..=2 => Err(format!(
                "Critical error: face can't have less than 3 points, but has {} points",
                vertex_indices.len()
            )),
            3 => {
                vertex_indices
                    .iter()
                    .for_each(|index| self.push_vertex_index(*index));
                uv_indices
                    .iter()
                    .for_each(|index| self.push_uv_index(*index));
                normal_indices
                    .iter()
                    .for_each(|index| self.push_normal_index(*index));
                Ok(())
            }
            _ => {
                #[cfg(feature = "triangulation")]
                {
                    let vertex_indices = SmallVec::from_slice(vertex_indices);
                    let normal_indices = SmallVec::from_slice(normal_indices);
                    let uv_indices = SmallVec::from_slice(uv_indices);
                    triangulate(self, vertex_indices, normal_indices, uv_indices);
                    return Ok(())
                }
                #[cfg(not(feature = "triangulation"))]
                return Err("Encountered a face that needed triangulation but experimental triangulation feature is disabled".to_owned())
            },
        }
    }
}
