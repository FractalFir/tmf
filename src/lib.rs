#![warn(missing_docs)]
#![warn(rustdoc::missing_doc_code_examples)]
//! **tmf** is a crate used to save and read 3D models saved in *.tmf* format. This format is focused on 2 things:
//! 1. Reducing size of the saved file as much as possible, without reducing visual quality
//! 2. Loading models as fast as possible(without sacrificing model size reduction)
//! This means that while saving a model may take a slightly longer time (4-6x loading), models can be loaded at considerable speed(Current estimation are around 25 milion vertices per second)
//! ## Feature flags
#![doc = document_features::document_features!()]
mod material;
mod normals;
mod obj;
/// Module used when saving vertex grups
mod pile_map;
mod tmf;
/// Module used to handle reads of data which is not bit aligned(for example, 3 or 17 bits). This is the module that allows for heavy compression used in this format.
#[doc(hidden)]
pub mod unaligned_rw;
mod utilis;
mod uv;
mod verify;
mod vertices;
// Unfinished

#[allow(dead_code)]
mod unaligned_lz;
const TMF_MAJOR: u16 = 0;
const TMF_MINOR: u16 = 1;
const MIN_TMF_MAJOR: u16 = 0;
const MIN_TMF_MINOR: u16 = 1;
/// Index type used for representing triangle indices.
#[cfg(not(any(feature = "long_indices", feature = "short_indices")))]
pub type IndexType = u32;
#[cfg(all(feature = "long_indices", feature = "short_indices"))]
compile_error!("Size of indices can't be both long(u64) or short(u16)");
#[cfg(feature = "long_indices")]
pub type IndexType = u64;
#[cfg(feature = "short_indices")]
pub type IndexType = u16;
/// Type used for representing floating-point numbers.
#[cfg(not(feature = "double_precision"))]
pub type FloatType = f32;
#[cfg(feature = "double_precision")]
pub type FloatType = f64;
/// Type used for representing 3d floating-point vectors
pub type Vector3 = (FloatType, FloatType, FloatType);
/// Type used for representing 2d floating-point vectors
pub type Vector2 = (FloatType, FloatType);
#[doc(inline)]
use crate::material::MaterialInfo;
pub use crate::normals::NormalPrecisionMode;
#[doc(inline)]
pub use crate::vertices::VertexPrecisionMode;
use std::io::{Read, Write};
#[doc(inline)]
pub use verify::TMFIntegrityStatus;
/// Settings for saving of a TMF mesh.
pub struct TMFPrecisionInfo {
    /// How much can the position of any vertex deviate, as a portion of the shortest edge in the model.
    pub vertex_precision: VertexPrecisionMode,
    /// How much can normal angle deviate, as an angle in degrees.
    pub normal_precision: NormalPrecisionMode,
    /// Do additional normal pruning before saving (has considerable performance impact if model has many vertices
    pub prune_normals: bool,
}
impl Default for TMFPrecisionInfo {
    /// Returns the default, middle-ground settings for saving meshes. Should be indistinguishable by human eye, but the LOD may be not enough for some rare cases (eg. procedural generation).
    fn default() -> Self {
        TMFPrecisionInfo {
            vertex_precision: VertexPrecisionMode(0.1),
            normal_precision: NormalPrecisionMode::from_deg_dev(1.0),
            prune_normals: true,
        }
    }
}
use std::io::Result;
/// Representation of a TMF mesh. Can be loaded from disk, imported from diffrent format, saved to disk, and exported to a diffrent format, or created using special functions. Since it can be user generated it may be invalid and **must** be verified before being saved, otherwise a "garbage" mesh may be saved, an error may occur or a panic may occur.
#[derive(Clone)]
pub struct TMFMesh {
    normals: Option<Box<[Vector3]>>,
    normal_faces: Option<Box<[IndexType]>>,
    vertices: Option<Box<[Vector3]>>,
    vertex_faces: Option<Box<[IndexType]>>,
    uvs: Option<Box<[Vector2]>>,
    uv_faces: Option<Box<[IndexType]>>,
    materials: Option<MaterialInfo>,
}
impl Default for TMFMesh {
    fn default() -> Self {
        Self::empty()
    }
}
fn vec_first<T: Sized + Clone>(vec: Vec<T>) -> T {
    vec[0].clone()
}
fn slice_to_box<T: Sized + std::marker::Copy>(slice: &[T]) -> Box<[T]> {
    slice.into()
}
impl TMFMesh {
    pub(crate) fn get_segment_count(&self) -> usize {
        let mut count = 0;
        //TODO: when adding new fields change this.
        if self.normals.is_some() {
            count += 1
        };
        if self.normal_faces.is_some() {
            count += 1
        };
        if self.vertices.is_some() {
            count += 1
        };
        if self.vertex_faces.is_some() {
            count += 1
        };
        if self.uvs.is_some() {
            count += 1
        };
        if self.uv_faces.is_some() {
            count += 1
        };
        if self.materials.is_some() {
            count += 1
        };
        count
    }
    /// Sets mesh vertex array and returns old vertex array if present. New mesh data is **not** checked during this function call, so to ensure mesh is valid call [`Self::verify`] before saving.
    ///```
    /// # use tmf::FloatType;
    /// # use tmf::TMFMesh;
    /// # let mut mesh = TMFMesh::empty();
    /// # let vertices = Vec::new();
    /// // Set the vertices of the mesh
    /// mesh.set_vertices(&vertices);
    ///```
    ///```
    /// # use tmf::FloatType;
    /// # fn do_something(_:&[(FloatType,FloatType,FloatType)]){}
    /// # let new_vertices = Vec::new();
    /// # let mut mesh = TMFMesh::empty();
    /// # mesh.set_vertices(&new_vertices);
    /// # use tmf::TMFMesh;
    /// // Change the vertices for some other vertices...
    /// let old_vertices = mesh.set_vertices(&new_vertices).expect("Mesh had no vertices!");
    /// // ... and the do something with old vertices
    /// do_something(&old_vertices);
    ///```
    pub fn set_vertices(&mut self, vertices: &[Vector3]) -> Option<Box<[Vector3]>> {
        let mut vertices = Some(slice_to_box(vertices));
        std::mem::swap(&mut vertices, &mut self.vertices);
        vertices
    }
    /// Sets mesh normal array and returns old normal array if present. New mesh data is **not** checked during this function call, so to ensure mesh is valid call `verify` before saving.
    ///```
    /// # use tmf::TMFMesh;
    /// // Set the normals of the mesh
    /// # let normals = Vec::new();
    /// # let mut mesh = TMFMesh::empty();
    /// mesh.set_normals(&normals);
    ///```
    ///```
    /// # fn do_something(_:&[(FloatType,FloatType,FloatType)]){}
    /// # use tmf::TMFMesh;
    /// # use tmf::FloatType;
    /// # let new_normals = Vec::new();
    /// # let mut mesh = TMFMesh::empty();
    /// # mesh.set_normals(&new_normals);
    /// // Change the normals  of this mesh for some other normals...
    /// let old_normals = mesh.set_normals(&new_normals).expect("Mesh had no normals!");
    /// // ... and the do something with old normals
    /// do_something(&old_normals);
    ///```
    pub fn set_normals(&mut self, normals: &[Vector3]) -> Option<Box<[Vector3]>> {
        let mut normals = Some(slice_to_box(normals));
        std::mem::swap(&mut normals, &mut self.normals);
        normals
    }
    /// Sets mesh uv array and returns old uv array if present. New mesh daata is **not** checked during this function call, so to ensure mesh is valid call [`Self::verify`] before saving.
    ///```
    /// # use tmf::FloatType;
    /// # fn do_something(_:&[(FloatType,FloatType)]){}
    /// # use tmf::TMFMesh;
    /// # let mut mesh = TMFMesh::empty();
    /// # let uvs = Vec::new();
    /// // Set the uvs of the mesh
    /// mesh.set_uvs(&uvs);
    ///```
    ///```
    /// # use tmf::FloatType;
    /// # use tmf::TMFMesh;
    /// # fn do_something(_:&[(FloatType,FloatType)]){}
    /// # let new_uvs = Vec::new();
    /// # let mut mesh = TMFMesh::empty();
    /// # mesh.set_uvs(&new_uvs);
    /// // Change the uvs  of this mesh for some other normals...
    /// let old_uvs = mesh.set_uvs(&new_uvs).expect("Mesh had no uvs!");
    /// // ... and the do something with old uvs
    /// do_something(&old_uvs);
    ///```
    pub fn set_uvs(&mut self, uvs: &[Vector2]) -> Option<Box<[Vector2]>> {
        let mut uvs = Some(slice_to_box(uvs));
        std::mem::swap(&mut uvs, &mut self.uvs);
        uvs
    }
    /// Sets vertex face array to *faces* and returns old faces if present.
    pub fn set_vertex_faces(&mut self, faces: &[IndexType]) -> Option<Box<[IndexType]>> {
        let mut faces = Some(slice_to_box(faces));
        std::mem::swap(&mut faces, &mut self.vertex_faces);
        faces
    }
    /// Sets normal face array to *faces* and returns old faces if present.
    pub fn set_normal_faces(&mut self, faces: &[IndexType]) -> Option<Box<[IndexType]>> {
        let mut faces = Some(slice_to_box(faces));
        std::mem::swap(&mut faces, &mut self.normal_faces);
        faces
    }
    /// Sets uv face array to *faces* and returns old faces if present.
    pub fn set_uv_faces(&mut self, faces: &[IndexType]) -> Option<Box<[IndexType]>> {
        let mut faces = Some(slice_to_box(faces));
        std::mem::swap(&mut faces, &mut self.uv_faces);
        faces
    }
    /// Gets the vertex array of this [`TMFMesh`].
    ///```
    /// # use tmf::TMFMesh;
    /// # let mesh = TMFMesh::empty();
    /// let vertices = mesh.get_vertices();
    ///```
    pub fn get_vertices(&self) -> Option<&[Vector3]> {
        match &self.vertices {
            Some(vertices) => Some(vertices.as_ref()),
            None => None,
        }
    }
    /// Gets the normal array of this [`TMFMesh`].
    ///```
    /// # use tmf::TMFMesh;
    /// # let mesh = TMFMesh::empty();
    /// let normals = mesh.get_normals();
    ///```
    pub fn get_normals(&self) -> Option<&[Vector3]> {
        match &self.normals {
            Some(normals) => Some(normals.as_ref()),
            None => None,
        }
    }
    /// Gets the uv array of this [`TMFMesh`].
    ///```
    /// # use tmf::TMFMesh;
    /// # let mesh = TMFMesh::empty();
    /// let uvs = mesh.get_uvs();
    ///```
    pub fn get_uvs(&self) -> Option<&[Vector2]> {
        match &self.uvs {
            Some(uvs) => Some(uvs.as_ref()),
            None => None,
        }
    }
    /// Gets the vertex face index array of this [`TMFMesh`].
    ///```
    /// # use tmf::TMFMesh;
    /// # let mesh = TMFMesh::empty();
    /// let vertex_faces = mesh.get_vertex_faces();
    ///```
    pub fn get_vertex_faces(&self) -> Option<&[IndexType]> {
        match &self.vertex_faces {
            Some(vertex_faces) => Some(vertex_faces.as_ref()),
            None => None,
        }
    }
    /// Gets the normal face index array of this [`TMFMesh`].
    ///```
    /// # use tmf::TMFMesh;
    /// # let mesh = TMFMesh::empty();
    /// let normal_faces = mesh.get_normal_faces();
    ///```
    pub fn get_normal_faces(&self) -> Option<&[IndexType]> {
        match &self.normal_faces {
            Some(normal_faces) => Some(normal_faces.as_ref()),
            None => None,
        }
    }
    /// Gets the uv face index array of this TMFMesh.
    ///```
    /// # use tmf::TMFMesh;
    /// # let mesh = TMFMesh::empty();
    /// let uv_faces = mesh.get_uv_faces();
    ///```
    pub fn get_uv_faces(&self) -> Option<&[IndexType]> {
        match &self.uv_faces {
            Some(uv_faces) => Some(uv_faces.as_ref()),
            None => None,
        }
    }
    /// Normalizes normal array, if present
    fn normalize(&mut self){
        /*
        use crate::normals::normalize_arr;
        match &self.normals{
            Some(mut normals) => normalize_arr(&mut normals),
            None=>(),
        }
        */
    }
    /// Checks if mesh is valid and can be saved.
    /// ```
    /// # use tmf::TMFMesh;
    /// # let mesh = TMFMesh::empty();
    /// // Get the tmf mesh form somewhere
    /// mesh.verify().expect("Mesh had errors!");
    /// ```
    pub fn verify(&self) -> std::result::Result<(), TMFIntegrityStatus> {
        verify::verify_tmf_mesh(self)
    }
    /// Reads tmf meshes from a .obj file in *reader*
    ///```
    /// # use tmf::TMFMesh;
    /// # fn do_something(_:TMFMesh,_:String){}
    /// # use std::fs::File;
    /// # let dot_obj_path = "testing/susan.obj";
    /// // Open the file with the .obj model
    /// let mut file = File::open(dot_obj_path).expect("Could not open .obj file!");
    /// // And multiple meshes from it
    /// let meshes = TMFMesh::read_from_obj(&mut file).expect("Could not parse .obj file!");
    /// for (mesh,name) in meshes{
    ///     // Do something with the mesh and name
    ///     do_something(mesh,name);
    /// }
    ///```
    pub fn read_from_obj<R: Read>(reader: &mut R) -> Result<Vec<(Self, String)>> {
        obj::read_from_obj(reader)
    }
    /// Reads a *single* named tmf mesh from a .obj file in *reader*, if more than one mesh present an error will be returned.
    ///```
    /// # use tmf::TMFMesh;
    /// # use std::fs::File;
    /// # let dot_obj_path = "testing/susan.obj";
    /// // Open the file with the .obj model
    /// let mut file = File::open(dot_obj_path).expect("Could not open .obj file!");
    /// // And read a mesh from it
    /// let (mesh,name) = TMFMesh::read_from_obj_one(&mut file).expect("Could not parse .obj file!");
    ///```
    pub fn read_from_obj_one<R: Read>(reader: &mut R) -> Result<(Self, String)> {
        let meshes = obj::read_from_obj(reader)?;
        if meshes.len() < 1 {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "No meshes present in .obj file",
            ))
        } else if meshes.len() > 1 {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "More than one mesh present in .tmf file while only one expected.",
            ))
        } else {
            Ok(vec_first(meshes))
        }
    }
    /// Writes this TMF  mesh to a .obj file.
    /// ``` 
    /// # use std::fs::File;
    /// # use tmf::{TMFMesh,TMFPrecisionInfo};
    /// # let mesh = TMFMesh::empty();
    /// # let out_path ="target/test_res/doc.obj";
    /// let mut obj_out = File::create(out_path).expect("Could not create obj out file!");
    /// mesh.write_obj_one(&mut obj_out,"mesh name").expect("Could not write the .obj file!");
    /// ```
    pub fn write_obj_one<W: Write>(&self, w: &mut W, name: &str) -> Result<()> {
        obj::write_obj(&[(self.clone(), name)], w)
    }
    /// Writes this TMF  mesh to a .obj file.
    pub fn write_obj<W: Write, S: std::borrow::Borrow<str>>(
        meshes: &[(TMFMesh, S)],
        w: &mut W,
    ) -> Result<()> {
        obj::write_obj(meshes, w)
    }
    /// Writes this TMF Mesh to *w*.
    pub fn write_tmf_one<W: Write>(
        &self,
        w: &mut W,
        p_info: &TMFPrecisionInfo,
        name: &str,
    ) -> Result<()> {
        tmf::write_tmf_header(w, 1)?;
        tmf::write_mesh(self, w, p_info, name)
    }
    /// Writes this TMF Mesh to *w*.
    pub fn write_tmf<W: Write>(
        meshes_names: &[(Self, &str)],
        w: &mut W,
        p_info: &TMFPrecisionInfo,
    ) -> Result<()> {
        tmf::write(meshes_names, w, p_info)
    }
    /// Creates an empty TMF Mesh.
    pub fn empty() -> Self {
        Self {
            normal_faces: None,
            normals: None,
            uv_faces: None,
            uvs: None,
            vertex_faces: None,
            vertices: None,
            materials: None,
        }
    }
    /// Reads all meshes from a .tmf file.
    pub fn read_tmf<R: Read>(reader: &mut R) -> Result<Vec<(Self, String)>> {
        Ok(tmf::read(reader)?)
    }
    /// Reads a single mesh from a .tmf file. Returns [`Err`] if no meshes present or more than one mesh present.
    pub fn read_tmf_one<R: Read>(reader: &mut R) -> Result<(Self, String)> {
        let meshes = Self::read_tmf(reader)?;
        if meshes.len() < 1 {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "No meshes present in .tmf file",
            ))
        } else if meshes.len() > 1 {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "More than one mesh present in .tmf file while only one expected.",
            ))
        } else {
            // TODO: find a way to remove this redundant clone call
            // This is a very stupid little "bug". Because 'meshes' must contain exactly 1 element(previous checks), the first element should just be returned.
            // This is in fact how it used to work, but after adding mesh names and changing the return type from TMFMesh to (TMFMesh,String) the function .nth(0) used
            // to get the first element started returning a reference for no apparent reason? So this less efficient way must suffice for now.This could be maybe fixed
            // with std::mem::swap, uninitialised dummy value and some fancy manual dropping but it would require using `unsafe` and could lead to a memory leak if done
            // incorrectly, so the clone call stays for now.
            Ok(vec_first(meshes))
        }
    }
}
#[cfg(test)]
mod testing {
    use super::*;
    fn init_test_env() {
        std::fs::create_dir_all("target/test_res").unwrap();
    }
    #[test]
    fn read_susan_obj() {
        init_test_env();
        let mut file = std::fs::File::open("testing/susan.obj").unwrap();
        let tmf_mesh = TMFMesh::read_from_obj_one(&mut file).unwrap().0;
        tmf_mesh.verify().unwrap();
    }
    #[test]
    fn rw_susan_obj() {
        init_test_env();
        let mut file = std::fs::File::open("testing/susan.obj").unwrap();
        let (tmf_mesh, name) = TMFMesh::read_from_obj_one(&mut file).unwrap();
        tmf_mesh.verify().unwrap();
        let mut out = std::fs::File::create("target/susan.obj").unwrap();
        tmf_mesh.write_obj_one(&mut out, &name).unwrap();
    }
    #[test]
    fn save_susan_tmf() {
        init_test_env();
        let mut file = std::fs::File::open("testing/susan.obj").unwrap();
        let (tmf_mesh, name) = TMFMesh::read_from_obj_one(&mut file).unwrap();
        tmf_mesh.verify().unwrap();
        let mut out = std::fs::File::create("target/test_res/susan.tmf").unwrap();
        assert!(name == "Suzanne");
        tmf_mesh
            .write_tmf_one(&mut out, &TMFPrecisionInfo::default(), &name)
            .unwrap();
    }
    #[test]
    fn rw_susan_tmf() {
        init_test_env();
        let mut file = std::fs::File::open("testing/susan.obj").unwrap();
        let (tmf_mesh, name) = TMFMesh::read_from_obj_one(&mut file).unwrap();
        tmf_mesh.verify().unwrap();
        assert!(name == "Suzanne");
        let mut out = Vec::new();
        {
            tmf_mesh
                .write_tmf_one(&mut out, &TMFPrecisionInfo::default(), "")
                .unwrap();
        }
        let (r_mesh, name) = TMFMesh::read_tmf_one(&mut (&out as &[u8])).unwrap();
        r_mesh.verify().unwrap();
        let mut out = std::fs::File::create("target/test_res/susan_ftmf.obj").unwrap();
        r_mesh.write_obj_one(&mut out, &name).unwrap();
    }
    #[test]
    #[should_panic]
    fn rw_cube_obj_not_triangulated() {
        init_test_env();
        let mut file = std::fs::File::open("testing/cube.obj").unwrap();
        let (tmf_mesh, name) = TMFMesh::read_from_obj_one(&mut file).unwrap();
        tmf_mesh.verify().unwrap();
        let mut out = std::fs::File::create("target/test_res/cube.obj").unwrap();
        tmf_mesh.write_obj_one(&mut out, &name).unwrap();
    }
    #[test]
    fn load_multpile_meshes_obj() {
        init_test_env();
        let mut file = std::fs::File::open("testing/multiple.obj").unwrap();
        let meshes = TMFMesh::read_from_obj(&mut file).unwrap();
        for mesh_name in meshes {
            let (mesh, name) = mesh_name;
            mesh.verify();
            let mut out = std::fs::File::create(&format!("target/test_res/{}.obj", name)).unwrap();
            mesh.write_obj_one(&mut out, &name).unwrap();
        }
    }
    #[test]
    fn rw_multpile_meshes_obj() {
        init_test_env();
        let mut file = std::fs::File::open("testing/multiple.obj").unwrap();
        let meshes = TMFMesh::read_from_obj(&mut file).unwrap();
        for mesh_name in &meshes {
            mesh_name.0.verify().unwrap();
        }
        let mut out = std::fs::File::create("target/test_res/multiple.obj").unwrap();
        TMFMesh::write_obj(&meshes, &mut out).unwrap();
    }
    #[test]
    fn read_multi_mtl_obj() {
        init_test_env();
        let mut file = std::fs::File::open("testing/multi_mtl.obj").unwrap();
        let tmf_mesh = TMFMesh::read_from_obj_one(&mut file).unwrap().0;
        tmf_mesh.verify().unwrap();
        todo!();
    }
    #[ignore]
    #[test]
    fn rw_60k_sph() {
        init_test_env();
        let mut file = std::fs::File::open("testing/60k.obj").unwrap();
        let tmf_mesh = TMFMesh::read_from_obj_one(&mut file).unwrap().0;
        tmf_mesh.verify().unwrap();
        let mut out = Vec::new();
        {
            tmf_mesh
                .write_tmf_one(&mut out, &TMFPrecisionInfo::default(), "")
                .unwrap();
        }
        let (r_mesh, name) = TMFMesh::read_tmf_one(&mut (&out as &[u8])).unwrap();
        r_mesh.verify().unwrap();
        let mut out = std::fs::File::create("target/60k_ftmf.obj").unwrap();
        r_mesh.write_obj_one(&mut out, &name).unwrap();
    }
    #[ignore]
    #[test]
    fn save_60k_sph_tmf() {
        init_test_env();
        let mut file = std::fs::File::open("testing/60k.obj").unwrap();
        let tmf_mesh = TMFMesh::read_from_obj_one(&mut file).unwrap().0;
        tmf_mesh.verify().unwrap();
        let mut out = std::fs::File::create("target/60k.tmf").unwrap();
        let mut prec = TMFPrecisionInfo::default();
        prec.prune_normals = false;
        tmf_mesh.write_tmf_one(&mut out, &prec, "").unwrap();
    }
}
