#![warn(missing_docs)]
#![deny(unused_must_use)]
//#![deny(dead_code)]
#![warn(rustdoc::missing_doc_code_examples)]
//! **tmf** is a crate used to save and read 3D models saved in *.tmf* format. This format is focused on 2 things:
//! 1. Reducing size of the saved file as much as possible, without reducing visual quality
//! 2. Loading models as fast as possible(without sacrificing model size reduction)
//! This means that while saving a model may take a slightly longer time (2-4x loading), models can be loaded at considerable speed(loading a model with around 40 000 points takes 1.6 ms)
//! ## Feature flags
//pub(crate) MAX_SEG_COUNT:usize = 0xFFFF;
#![doc = document_features::document_features!()]
#[doc(hidden)]
pub mod custom_data;
mod material;
#[cfg(feature = "model_importer")]
mod model_importer;
mod normals;
#[cfg(feature = "obj_import")]
mod obj;
#[doc(hidden)]
pub mod tangents;
mod tmf;
mod tmf_exporter;
mod tmf_importer;
/// Module used to handle reads of data which is not bit aligned(for example, 3 or 17 bits). This is the module that allows for heavy compression used in this format.
#[doc(hidden)]
pub mod unaligned_rw;
mod utilis;
mod uv;
mod verify;
mod vertices;
// Unfinished
mod lz77;
const TMF_MAJOR: u16 = 0;
const TMF_MINOR: u16 = 2;
const MIN_TMF_MAJOR: u16 = 0;
const MIN_TMF_MINOR: u16 = 2;
pub(crate) const MAX_SEG_SIZE: usize = 0x80_00_00_00; // 2_00_00 for fuzzing!
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
use crate::custom_data::CustomDataSegment;
#[doc(inline)]
pub use crate::custom_data::{CustomData, DataSegmentError};
#[doc(inline)]
use crate::material::MaterialInfo;
#[doc(inline)]
pub use crate::normals::NormalPrecisionMode;
#[doc(inline)]
pub use crate::tangents::*;
#[doc(inline)]
pub use crate::uv::UvPrecisionMode;
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
    /// How much can saved UVs deviate.
    pub uv_prec: crate::UvPrecisionMode,
    /// How much can saved tangents deviate
    pub tangent_prec: TangentPrecisionMode,
}
impl Default for TMFPrecisionInfo {
    /// Returns the default, middle-ground settings for saving meshes. Should be indistinguishable by human eye, but the LOD may be not enough for some rare cases (eg. procedural generation).
    fn default() -> Self {
        TMFPrecisionInfo {
            vertex_precision: VertexPrecisionMode::default(),
            normal_precision: NormalPrecisionMode::default(),
            uv_prec: crate::UvPrecisionMode::default(),
            tangent_prec: TangentPrecisionMode::default(),
        }
    }
}
/// Representation of a TMF mesh. Can be loaded from disk, imported from diffrent format, saved to disk, and exported to a diffrent format, or created using special functions. Any mesh created at run time *should* but does not *have to* be checked before saving with [`Self::verify`] call. If the mesh is known to be OK before saving this step can be skipped(even tough it is still advised).
#[derive(Clone)]
pub struct TMFMesh {
    normals: Option<Vec<Vector3>>,
    normal_triangles: Option<Vec<IndexType>>,
    vertices: Option<Vec<Vector3>>,
    vertex_triangles: Option<Vec<IndexType>>,
    uvs: Option<Vec<Vector2>>,
    uv_triangles: Option<Vec<IndexType>>,
    tangents: Option<Vec<Tangent>>,
    tangent_triangles: Option<Vec<IndexType>>,
    materials: Option<MaterialInfo>,
    custom_data: Vec<CustomDataSegment>,
}
impl Default for TMFMesh {
    /// Creates default, empty [`TMFMesh`]. Equivalent to [`TMFMesh::empty`] call.
    fn default() -> Self {
        Self::empty()
    }
}
impl TMFMesh {
    pub fn optimize(&mut self) {
        /*
        if let Some((vertices, vertex_triangles)) =
            self.get_vertices().zip(self.get_vertex_triangles())
        {
            let (vertex_triangles, vertices) =
                utilis::optimize_triangle_indices(vertex_triangles, vertices);
            self.set_vertices(vertices);
            self.set_vertex_triangles(vertex_triangles);
        }
        if let Some((normals, normal_triangles)) =
            self.get_normals().zip(self.get_normal_triangles())
        {
            let (normal_triangles, normals) =
                utilis::optimize_triangle_indices(normal_triangles, normals);
            self.set_normals(normals);
            self.set_normal_triangles(normal_triangles);
        }
        if let Some((uvs, uv_triangles)) =
            self.get_uvs().zip(self.get_uv_triangles())
        {
            let (uv_triangles, uvs) =
                utilis::optimize_triangle_indices(uv_triangles, uvs);
            self.set_uvs(uvs);
            self.set_uv_triangles(uv_triangles);
        }*/
    }
    /// Sets mesh vertex array and returns old vertex array if present. New mesh data is **not** checked during this function call, so to ensure mesh is valid call [`Self::verify`] before saving.
    ///```
    /// # use tmf::FloatType;
    /// # use tmf::TMFMesh;
    /// # let mut mesh = TMFMesh::empty();
    /// # let vertices = Vec::new();
    /// // Set the vertices of the mesh
    /// mesh.set_vertices(vertices);
    ///```
    ///```
    /// # use tmf::FloatType;
    /// # fn do_something(_:&[(FloatType,FloatType,FloatType)]){}
    /// # let new_vertices = Vec::new();
    /// # let mut mesh = TMFMesh::empty();
    /// # mesh.set_vertices(new_vertices.clone());
    /// # use tmf::TMFMesh;
    /// // Change the vertices for some other vertices...
    /// let old_vertices = mesh.set_vertices(new_vertices).expect("Mesh had no vertices!");
    /// // ... and the do something with old vertices
    /// do_something(&old_vertices);
    ///```
    pub fn set_vertices<T: Into<Vec<Vector3>>>(&mut self, vertices: T) -> Option<Vec<Vector3>> {
        self.vertices.replace(vertices.into())
    }
    /// Sets mesh normal array and returns old normal array if present. New mesh data is **not** checked during this function call, so to ensure mesh is valid call `verify` before saving.
    ///```
    /// # use tmf::TMFMesh;
    /// // Set the normals of the mesh
    /// # let normals = Vec::new();
    /// # let mut mesh = TMFMesh::empty();
    /// mesh.set_normals(normals);
    ///```
    ///```
    /// # fn do_something(_:&[tmf::Vector3]){}
    /// # use tmf::TMFMesh;
    /// # use tmf::FloatType;
    /// # let new_normals = Vec::new();
    /// # let mut mesh = TMFMesh::empty();
    /// # mesh.set_normals(new_normals.clone());
    /// // Change the normals  of this mesh for some other normals...
    /// let old_normals = mesh.set_normals(new_normals).expect("Mesh had no normals!");
    /// // ... and the do something with old normals
    /// do_something(&old_normals);
    ///```
    pub fn set_normals<T: Into<Vec<Vector3>>>(&mut self, normals: T) -> Option<Vec<Vector3>> {
        self.normals.replace(normals.into())
    }
    /// Sets mesh tangent array and returns old tangent array if present. New mesh data is **not** checked during this function call, so to ensure mesh is valid call `verify` before saving.
    ///```
    /// # use tmf::TMFMesh;
    /// // Set the tangents of the mesh
    /// # let tangents = Vec::new();
    /// # let mut mesh = TMFMesh::empty();
    /// mesh.set_tangents(tangents);
    ///```
    ///```
    /// # fn do_something(_:&[tmf::Tangent]){}
    /// # use tmf::TMFMesh;
    /// # use tmf::FloatType;
    /// # let new_tangents = Vec::new();
    /// # let mut mesh = TMFMesh::empty();
    /// # mesh.set_tangents(new_tangents.clone());
    /// // Change the tangents  of this mesh for some other tangents...
    /// let old_tangents = mesh.set_tangents(new_tangents).expect("Mesh had no tangents!");
    /// // ... and the do something with old tangents
    /// do_something(&old_tangents);
    ///```
    pub fn set_tangents<T: Into<Vec<Tangent>>>(&mut self, tangents: T) -> Option<Vec<Tangent>> {
        self.tangents.replace(tangents.into())
    }
    /// Sets mesh uv array and returns old uv array if present. New mesh daata is **not** checked during this function call, so to ensure mesh is valid call [`Self::verify`] before saving.
    ///```
    /// # use tmf::FloatType;
    /// # fn do_something(_:&[(FloatType,FloatType)]){}
    /// # use tmf::TMFMesh;
    /// # let mut mesh = TMFMesh::empty();
    /// # let uvs = Vec::new();
    /// // Set the uvs of the mesh
    /// mesh.set_uvs(uvs);
    ///```
    ///```
    /// # use tmf::FloatType;
    /// # use tmf::TMFMesh;
    /// # fn do_something(_:&[(FloatType,FloatType)]){}
    /// # let mut mesh = TMFMesh::empty();
    /// # let new_uvs = Vec::new();
    /// # mesh.set_uvs(new_uvs.clone());
    /// // Change the uvs  of this mesh for some other normals...
    /// let old_uvs = mesh.set_uvs(new_uvs).expect("Mesh had no uvs!");
    /// // ... and the do something with old uvs
    /// do_something(&old_uvs);
    ///```
    pub fn set_uvs<T: Into<Vec<Vector2>>>(&mut self, uvs: T) -> Option<Vec<Vector2>> {
        self.uvs.replace(uvs.into())
    }
    /// Sets vertex index array to *triangles* and returns old triangles if present.
    ///```
    /// # use tmf::TMFMesh;
    /// # let mut mesh = TMFMesh::empty();
    /// # let triangles = [0,1,2,3,2,1];
    /// mesh.set_vertex_triangles(triangles);
    ///```
    pub fn set_vertex_triangles<T: Into<Vec<IndexType>>>(
        &mut self,
        triangles: T,
    ) -> Option<Vec<IndexType>> {
        self.vertex_triangles.replace(triangles.into())
    }
    /// Sets normal index array to *triangles* and returns old triangles if present.
    ///```
    /// # use tmf::TMFMesh;
    /// # let mut mesh = TMFMesh::empty();
    /// # let triangles = [0,1,2,3,2,1];
    /// mesh.set_normal_triangles(triangles);
    ///```
    pub fn set_normal_triangles<T: Into<Vec<IndexType>>>(
        &mut self,
        triangles: T,
    ) -> Option<Vec<IndexType>> {
        self.normal_triangles.replace(triangles.into())
    }
    /// Sets uv index array to *triangles* and returns old triangles if present.
    ///```
    /// # use tmf::TMFMesh;
    /// # let mut mesh = TMFMesh::empty();
    /// # let triangles = [0,1,2,3,2,1];
    /// mesh.set_uv_triangles(triangles);
    ///```
    pub fn set_uv_triangles<T: Into<Vec<IndexType>>>(
        &mut self,
        triangles: T,
    ) -> Option<Vec<IndexType>> {
        self.uv_triangles.replace(triangles.into())
    }
    /// Sets tangent index array to *triangles* and returns old triangles if present.
    ///```
    /// # use tmf::TMFMesh;
    /// # let mut mesh = TMFMesh::empty();
    /// # let triangles = [0,1,2,3,2,1];
    /// mesh.set_tangent_triangles(triangles);
    ///```
    pub fn set_tangent_triangles<T: Into<Vec<IndexType>>>(
        &mut self,
        triangles: T,
    ) -> Option<Vec<IndexType>> {
        self.tangent_triangles.replace(triangles.into())
    }
    /// Gets the vertex array of this [`TMFMesh`].
    ///```
    /// # use tmf::TMFMesh;  
    /// # let mesh = TMFMesh::empty();
    /// let vertices = mesh.get_vertices();
    ///```
    #[must_use]
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
    #[must_use]
    pub fn get_normals(&self) -> Option<&[Vector3]> {
        match &self.normals {
            Some(normals) => Some(normals.as_ref()),
            None => None,
        }
    }
    /// Gets the tangent array of this [`TMFMesh`].
    ///```
    /// # use tmf::TMFMesh;
    /// # let mesh = TMFMesh::empty();
    /// let normals = mesh.get_tangents();
    ///```
    #[must_use]
    pub fn get_tangents(&self) -> Option<&[Tangent]> {
        match &self.tangents {
            Some(tangents) => Some(tangents.as_ref()),
            None => None,
        }
    }
    /// Gets the uv array of this [`TMFMesh`].
    ///```
    /// # use tmf::TMFMesh;
    /// # let mesh = TMFMesh::empty();
    /// let uvs = mesh.get_uvs();
    ///```
    #[must_use]
    pub fn get_uvs(&self) -> Option<&[Vector2]> {
        match &self.uvs {
            Some(uvs) => Some(uvs.as_ref()),
            None => None,
        }
    }
    /// Gets the vertex triangle index array of this [`TMFMesh`].
    ///```
    /// # use tmf::TMFMesh;
    /// # let mesh = TMFMesh::empty();
    /// let vertex_triangles = mesh.get_vertex_triangles();
    ///```
    #[must_use]
    pub fn get_vertex_triangles(&self) -> Option<&[IndexType]> {
        match &self.vertex_triangles {
            Some(vertex_triangles) => Some(vertex_triangles.as_ref()),
            None => None,
        }
    }
    /// Gets the normal triangle index array of this [`TMFMesh`].
    ///```
    /// # use tmf::TMFMesh;
    /// # let mesh = TMFMesh::empty();
    /// let normal_triangles = mesh.get_normal_triangles();
    ///```
    #[must_use]
    pub fn get_normal_triangles(&self) -> Option<&[IndexType]> {
        match &self.normal_triangles {
            Some(normal_triangles) => Some(normal_triangles.as_ref()),
            None => None,
        }
    }
    /// Gets the uv triangle index array of this [`TMFMesh`].
    ///```
    /// # use tmf::TMFMesh;
    /// # let mesh = TMFMesh::empty();
    /// let uv_triangles = mesh.get_uv_triangles();
    ///```
    #[must_use]
    pub fn get_uv_triangles(&self) -> Option<&[IndexType]> {
        match &self.uv_triangles {
            Some(uv_triangles) => Some(uv_triangles.as_ref()),
            None => None,
        }
    }
    /// Gets the tangent triangle index array of this [`TMFMesh`].
    ///```
    /// # use tmf::TMFMesh;
    /// # let mesh = TMFMesh::empty();
    /// let tangent_triangles = mesh.get_tangent_triangles();
    ///```
    #[must_use]
    pub fn get_tangent_triangles(&self) -> Option<&[IndexType]> {
        match &self.tangent_triangles {
            Some(tangent_triangles) => Some(tangent_triangles.as_ref()),
            None => None,
        }
    }
    /// Returns array containing points laid out in such a way that each 3 points create the next triangle.
    /// If mesh has no vertex array or no vertex triangle array [`None`] is returned.
    ///```
    /// # use tmf::TMFMesh;
    /// # let mut mesh = TMFMesh::empty();
    /// # let vertices = [(0.0,0.0,0.0),(1.0,0.0,0.0),(1.0,1.0,0.0),(0.0,1.0,0.0)];
    /// # let vertex_triangles = [0,1,2,0,2,3];
    /// # mesh.set_vertices(vertices);
    /// # mesh.set_vertex_triangles(vertex_triangles);
    /// let vert_buff = mesh.get_vertex_buffer().expect("Could not create the array of points creating triangles!");
    /// // The same number of triangles created by points and triangles created by indices
    /// assert!(vert_buff.len() == vertex_triangles.len());
    ///```
    #[must_use]
    pub fn get_vertex_buffer(&self) -> Option<Box<[Vector3]>> {
        let vertices = self.get_vertices()?;
        let triangles = self.get_vertex_triangles()?;
        let mut vertex_buffer = Vec::with_capacity(triangles.len());
        for index in triangles {
            match vertices.get(*index as usize) {
                Some(vertex) => vertex_buffer.push(*vertex),
                None => return None, //panic!("Invalid TMFMesh: vertex index outside vertex array!"),
            }
        }
        Some(vertex_buffer.into())
    }
    /// Returns array containing normals laid out in such a way that each 3 normals create the next triangle.
    /// If mesh has no normal array or no normal triangle array [`None`] is returned.
    ///```
    /// # use tmf::TMFMesh;
    /// # let mut mesh = TMFMesh::empty();
    /// # let normals = [(0.0,0.0,0.0),(1.0,0.0,0.0),(1.0,1.0,0.0),(0.0,1.0,0.0)];
    /// # let normal_triangles = [0,1,2,0,2,3];
    /// # mesh.set_normals(normals);
    /// # mesh.normalize();
    /// # mesh.set_normal_triangles(normal_triangles);
    /// let normal_buff = mesh.get_normal_buffer().expect("Could not create the array of normals creating triangles!");
    /// // The same number of triangles created by points and triangles created by indices
    /// assert!(normal_buff.len() == normal_triangles.len());
    ///```
    #[must_use]
    pub fn get_normal_buffer(&self) -> Option<Box<[Vector3]>> {
        let normals = self.get_normals()?;
        let triangles = self.get_normal_triangles()?;
        let mut normals_buffer = Vec::with_capacity(triangles.len());
        for index in triangles {
            match normals.get(*index as usize) {
                Some(normal) => normals_buffer.push(*normal),
                None => return None, //panic!("Invalid TMFMesh: normal index outside vertex array!"),
            }
        }
        Some(normals_buffer.into())
    }
    /// Returns array containing UV coridnates laid out in such a way that each 3 cordiantes create the next triangle.
    /// If mesh has no UV array or no uv triangle array [`None`] is returned.
    ///```
    /// # use tmf::TMFMesh;
    /// # let mut mesh = TMFMesh::empty();
    /// # let uvs = [(0.0,0.0),(1.0,0.0),(1.0,1.0),(0.0,1.0)];
    /// # let uv_triangles = [0,1,2,0,2,3];
    /// # mesh.set_uvs(uvs);
    /// # mesh.set_uv_triangles(uv_triangles);
    /// let uv_buff = mesh.get_uv_buffer().expect("Could not create the array of uvs creating triangles!");
    /// // The same number of triangles created by points and triangles created by indices
    /// assert!(uv_buff.len() == uv_triangles.len());
    ///```
    #[must_use]
    pub fn get_uv_buffer(&self) -> Option<Box<[Vector2]>> {
        let uvs = self.get_uvs()?;
        let triangles = self.get_uv_triangles()?;
        let mut uv_buffer = Vec::with_capacity(triangles.len());
        for index in triangles {
            match uvs.get(*index as usize) {
                Some(uv) => uv_buffer.push(*uv),
                None => return None, //panic!("Invalid TMFMesh: uv index outside vertex array!"),
            }
        }
        Some(uv_buffer.into())
    }
    /// Normalizes normal array of the mesh, if it is present.
    ///```
    /// # use tmf::{TMFMesh,FloatType,Vector3};
    /// # let normals = vec![(0.2,3.4,4.3),(-5.4,1.412,3.32),(5.5,-2.1,-2.3)];
    /// # let mut mesh = TMFMesh::empty();
    /// # pub(crate) fn magnitude(i: Vector3) -> FloatType
    /// # {let xx = i.0 * i.0;let yy = i.1 * i.1;let zz = i.2 * i.2;(xx + yy + zz).sqrt()}
    /// // Some mesh with normals which are not normalzed
    /// mesh.set_normals(normals);
    /// // Normalize all normals
    /// mesh.normalize();
    /// // All normals are normalised (their magnitude is equal to 1)
    /// for normal in mesh.get_normals().expect("Mesh has no normals"){
    ///     let mag = magnitude(*normal);
    ///     // Magnitude of the normal is not exactly equal to 1
    ///     // because of imprecision of floating-point numbers.
    ///     assert!(mag > 0.99999 && mag < 1.00001);
    /// }
    ///```
    /// Normalising a mesh without normals is a NOP.
    ///```
    /// # use tmf::TMFMesh;
    /// # let mut mesh = TMFMesh::empty();
    /// // Mesh has no normals
    /// assert!(mesh.get_normals().is_none());
    /// // This does nothing
    /// mesh.normalize();
    ///```
    pub fn normalize(&mut self) {
        use crate::normals::normalize_arr;
        let normals = self.normals.as_mut();
        if let Some(normals) = normals {
            normalize_arr(normals);
        };
    }
    /// Checks if mesh is valid and can be saved.
    /// ```
    /// # use tmf::TMFMesh;
    /// # let mesh = TMFMesh::empty();
    /// // Get the tmf mesh form somewhere
    /// mesh.verify().expect("Mesh had errors!");
    /// ```
    /// # Errors
    /// Returns a [`TMFIntegrityStatus`] if mesh is invalid.
    pub fn verify(&self) -> std::result::Result<(), TMFIntegrityStatus> {
        verify::verify_tmf_mesh(self)
    }
    /// Reads tmf meshes from a .obj file in *reader*
    /// In order to enable triangulation while importing .obj files feature triangulation must be used. It is still highly experimental so read documentation before enabling.
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
    /// # Errors
    /// Returns IO error if it occurs.
    #[cfg(feature = "obj_import")]
    pub fn read_from_obj<R: Read>(reader: &mut R) -> std::io::Result<Vec<(Self, String)>> {
        obj::read_from_obj(reader)
    }
    /// Reads a *single* named tmf mesh from a .obj file in *reader*, if more than one mesh present an error will be returned.
    /// In order to enable triangulation while importing .obj files feature triangulation must be used. It is still highly experimental so read documentation before enabling.
    ///```
    /// # use tmf::TMFMesh;
    /// # use std::fs::File;
    /// # let dot_obj_path = "testing/susan.obj";
    /// // Open the file with the .obj model
    /// let mut file = File::open(dot_obj_path).expect("Could not open .obj file!");
    /// // And read a mesh from it
    /// let (mesh,name) = TMFMesh::read_from_obj_one(&mut file).expect("Could not parse .obj file!");
    ///```
    /// # Errors
    /// Returns IO error if it occurs, or wrong mesh count.
    #[cfg(feature = "obj_import")]
    pub fn read_from_obj_one<R: Read>(reader: &mut R) -> std::io::Result<(Self, String)> {
        let mut meshes = obj::read_from_obj(reader)?.into_iter();
        match meshes.next() {
            Some(mesh) => {
                if meshes.next().is_some() {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "More than one mesh present in .obj file while only one expected.",
                    ))
                } else {
                    Ok(mesh)
                }
            }
            None => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "No meshes present in .obj file",
            )),
        }
    }
    /// Writes this TMF  mesh to a .obj file.
    /// ```
    /// # use std::fs::File;
    /// # use tmf::TMFMesh;
    /// # let mesh = TMFMesh::empty();
    /// # let out_path ="target/test_res/doc.obj";
    /// let mut obj_out = File::create(out_path).expect("Could not create obj out file!");
    /// mesh.write_obj_one(&mut obj_out,"mesh name").expect("Could not write the .obj file!");
    /// ```
    /// # Errors
    /// Returns IO error if it occurs.
    #[cfg(feature = "obj_import")]
    pub fn write_obj_one<W: Write>(&self, w: &mut W, name: &str) -> std::io::Result<()> {
        obj::write_obj(&[(self.clone(), name)], w)
    }
    /// Writes multiple TMF meshes to a .obj file.
    ///```
    /// # use std::fs::File;
    /// # use tmf::{TMFMesh,TMFPrecisionInfo};
    /// # let meshes = [(TMFMesh::empty(),"a".to_owned()),(TMFMesh::empty(),"b".to_owned())];
    /// # let path = "target/test_res/doc_multiple_out.obj";
    /// let mut output = File::create(path).expect("Could not create file!");
    /// TMFMesh::write_obj(&meshes,&mut output).expect("Could not export to .obj");
    ///```
    /// # Errors
    /// Returns IO error if it occurs.
    #[cfg(feature = "obj_import")]
    pub fn write_obj<W: Write, S: std::borrow::Borrow<str>>(
        meshes: &[(TMFMesh, S)],
        w: &mut W,
    ) -> std::io::Result<()> {
        obj::write_obj(meshes, w)
    }
    /// Writes this TMF Mesh to *w*.
    ///```
    /// # use std::fs::File;
    /// # use tmf::{TMFMesh,TMFPrecisionInfo};
    /// # let mesh = TMFMesh::empty();
    /// # let path = "target/test_res/doc_out.tmf";
    /// let mut output = File::create(path).expect("Could not create file!");
    /// // Settings dictating how to save the mesh
    /// let precision_info = TMFPrecisionInfo::default();
    /// mesh.write_tmf_one(&mut output,&precision_info,"mesh_name").expect("Could not save .tmf mesh!");;
    ///```
    /// # Errors
    /// Returns IO error if occurs.
    pub fn write_tmf_one<W: Write, S: std::borrow::Borrow<str>>(
        &self,
        w: &mut W,
        p_info: &TMFPrecisionInfo,
        name: S,
    ) -> Result<(), TMFExportError> {
        futures::executor::block_on(tmf_exporter::write_tmf(&[(self.clone(), name)], w, p_info))
    }
    /// Writes a number of TMF meshes into one file.
    /// ```
    /// # use std::fs::File;
    /// # use tmf::{TMFMesh,TMFPrecisionInfo};
    /// # let meshes = [(TMFMesh::empty(),"a".to_owned()),(TMFMesh::empty(),"b".to_owned())];
    /// # let path = "target/test_res/doc_multiple_out.tmf";
    /// let mut output = File::create(path).expect("Could not create file!");
    /// // Settings dictating how to save the mesh
    /// let precision_info = TMFPrecisionInfo::default();
    /// TMFMesh::write_tmf(&meshes,&mut output, &precision_info).expect("Could not save .tmf file!");
    /// ```
    /// # Errors
    /// Returns IO error if occurs.
    pub fn write_tmf<W: Write, S: std::borrow::Borrow<str>>(
        meshes_names: &[(Self, S)],
        w: &mut W,
        p_info: &TMFPrecisionInfo,
    ) -> Result<(), TMFExportError> {
        futures::executor::block_on(tmf_exporter::write_tmf(meshes_names, w, p_info))
    }
    /// Creates an empty TMF Mesh(mesh with no data). Equivalent to [`TMFMesh::default`].
    /// ```
    /// # use tmf::TMFMesh;
    /// // Creates an empty mesh with no data
    /// let mut mesh = TMFMesh::empty();
    /// ```
    #[must_use]
    pub fn empty() -> Self {
        Self {
            normal_triangles: None,
            normals: None,
            uv_triangles: None,
            uvs: None,
            vertex_triangles: None,
            vertices: None,
            tangents: None,
            materials: None,
            custom_data: Vec::new(),
            tangent_triangles: None,
        }
    }
    /// Reads all meshes from a .tmf file.
    /// ```
    /// # use std::fs::File;
    /// # use tmf::{TMFMesh,TMFPrecisionInfo};
    /// # let tmf_path = "testing/susan.tmf";
    /// # fn do_something(_:TMFMesh,_:String){}
    /// // Open file containg the meshes
    /// let mut file = File::open(tmf_path).expect("Could not open .tmf file");
    /// // Get meshes and their names
    /// let meshes = TMFMesh::read_tmf(&mut file).expect("Could not load .tmf mesh!");
    /// for (mesh,name) in meshes{
    ///     // Do some thing with each mesh and name(eg. use in game)
    ///     do_something(mesh,name);
    /// }
    /// ```
    /// # Errors
    /// Returns: an IO error if it occurs, `NotTMFFile` if not tmf file, `NewerVersionRequired` if a newer importer is required for importing the file, and other errors for malformed tmf files.
    pub fn read_tmf<R: Read>(reader: &mut R) -> Result<Vec<(Self, String)>, TMFImportError> {
        crate::tmf_importer::import_sync(reader)
    }
    /// Reads a single mesh from a .tmf file. Returns [`Err`] if no meshes present or more than one mesh present.
    /// ```
    /// # use tmf::TMFMesh;
    /// # use std::fs::File;
    /// # let tmf_path = "testing/susan.tmf";
    /// // Open the file containing the mesh
    /// let mut file = File::open(tmf_path).expect("Could not open .tmf file");
    /// // Read mesh and mesh name form file
    /// let (mesh,name) = TMFMesh::read_tmf_one(&mut file).expect("Could not load .tmf mesh!");
    /// ```
    /// # Errors
    /// Returns: an IO error if it occurs, `NotTMFFile` if not tmf file, `NewerVersionRequired` if a newer importer is required for importing the file, and other errors for malformed tmf files. This function also returns `NoMeshes` or `TooManyMeshes` if wrong mesh count present.
    pub fn read_tmf_one<R: Read>(reader: &mut R) -> Result<(Self, String), TMFImportError> {
        let mut meshes = Self::read_tmf(reader)?.into_iter();
        match meshes.next() {
            Some(mesh) => {
                if meshes.next().is_some() {
                    Err(TMFImportError::TooManyMeshes)
                } else {
                    Ok(mesh)
                }
            }
            None => Err(TMFImportError::NoMeshes),
        }
    }
    /// Adds custom data array.
    /// # Errors
    /// Returns `Err` if name is too long (over 255 bytes) or empty.
    pub fn add_custom_data(
        &mut self,
        custom_data: CustomData,
        name: &str,
    ) -> Result<(), DataSegmentError> {
        self.add_custom_data_seg(CustomDataSegment::new(custom_data, name)?);
        Ok(())
    }
    pub(crate) fn add_custom_data_seg(&mut self, custom_data: CustomDataSegment) {
        self.custom_data.push(custom_data);
    }
    /// Gets a custom data array with name *name*.
    /// Returns `None`, if data not present, or name too long(over 255 bytes).
    #[must_use]
    pub fn lookup_custom_data(&self, name: &str) -> Option<&CustomData> {
        let bytes = name.as_bytes();
        if bytes.len() > u8::MAX as usize {
            return None;
        }
        let Ok(bytes_len) = u8::try_from(bytes.len()) else { return None };
        for data in &self.custom_data {
            if data.name_len() == bytes_len
                && bytes == &data.name_bytes()[..(data.name_len() as usize)]
            {
                return Some(data.custom_data());
            }
        }
        None
    }
    /// Appends vertices to this meshes vertex array.
    pub fn append_vertices(&mut self, vertices: &[Vector3]) {
        match &mut self.vertices {
            Some(ref mut self_v) => self_v.extend(vertices),
            None => {
                self.set_vertices(vertices);
            }
        };
    }
    /// Appends normals to this meshes normal array.
    pub fn append_normals(&mut self, normals: &[Vector3]) {
        match &mut self.normals {
            Some(ref mut self_n) => self_n.extend(normals),
            None => {
                self.set_normals(normals);
            }
        };
    }
    /// Appends tangents to this meshes tangent array.
    pub fn append_tangents(&mut self, tangents: &[Tangent]) {
        match &mut self.tangents {
            Some(ref mut self_t) => self_t.extend(tangents),
            None => {
                self.set_tangents(tangents);
            }
        };
    }
    /// Appends uvs to this meshes uv array.
    pub fn append_uvs(&mut self, uvs: &[Vector2]) {
        match &mut self.uvs {
            Some(ref mut self_uv) => self_uv.extend(uvs),
            None => {
                self.set_uvs(uvs);
            }
        };
    }
    /// Appends indices to this meshes vertex triangle array.
    pub fn append_vertex_triangles(&mut self, triangles: &[IndexType]) {
        match &mut self.vertex_triangles {
            Some(ref mut self_vt) => self_vt.extend(triangles),
            None => {
                self.set_vertex_triangles(triangles);
            }
        };
    }
    /// Appends indices to this meshes normal triangle array.
    pub fn append_normal_triangles(&mut self, triangles: &[IndexType]) {
        match &mut self.normal_triangles {
            Some(ref mut self_nt) => self_nt.extend(triangles),
            None => {
                self.set_normal_triangles(triangles);
            }
        };
    }
    /// Appends indices to this meshes uv triangle array.
    pub fn append_uv_triangles(&mut self, triangles: &[IndexType]) {
        match &mut self.uv_triangles {
            Some(ref mut self_uvt) => self_uvt.extend(triangles),
            None => {
                self.set_uv_triangles(triangles);
            }
        };
    }
    /// Appends indices to this meshes uv triangle array.
    pub fn append_tangent_triangles(&mut self, triangles: &[IndexType]) {
        match &mut self.tangent_triangles {
            Some(ref mut self_tant) => self_tant.extend(triangles),
            None => {
                self.set_tangent_triangles(triangles);
            }
        };
    }
}
/// An enum describing an error that occurred during loading a TMF mesh.  
#[derive(Debug)]
pub enum TMFImportError {
    /// An IO error which prevented data from being read.
    IO(std::io::Error),
    /// A segment uses an unknown compression type, invalid for the minimum TMF version specified by file header.
    CompressionTypeUnknown(u8),
    /// A method which should return one mesh was called, but TMF file had no meshes present.
    NoMeshes,
    /// A method which should return one mesh was called, but more than one mesh was present.
    TooManyMeshes,
    /// Provides source was not a TMF file.
    NotTMFFile,
    /// File was created with a TMF version newer than this, and can't be read properly.
    NewerVersionRequired,
    /// A file segment exceeded the maximum length(2GB) was encountered. This segments length is highly unusual, and the segment unlikely to be valid. The segment was not read to prevent memory issues.
    SegmentTooLong,
    /// A segments compression type requires that it must be preceded by another segment, from which some of the data is deduced.   
    NoDataBeforeOmmitedSegment,
    /// Byte precision is too high(over 64 bits) and is invalid.
    InvalidPrecision(u8),
}
impl From<std::io::Error> for TMFImportError {
    fn from(err: std::io::Error) -> Self {
        Self::IO(err)
    }
}
/// An error which occured when a `TMFMesh` is exported.
#[derive(Debug)]
pub enum TMFExportError {
    /// An IO error which prevented data from being read.
    IO(std::io::Error),
}
impl From<std::io::Error> for TMFExportError {
    fn from(err: std::io::Error) -> Self {
        Self::IO(err)
    }
}
#[cfg(test)]
pub(crate) fn init_test_env() {
    std::fs::create_dir_all("target/test_res").unwrap();
}
#[cfg(test)]
mod testing {
    use super::*;

    #[test]
    #[cfg(feature = "obj_import")]
    fn read_susan_obj() {
        init_test_env();
        let mut file = std::fs::File::open("testing/susan.obj").unwrap();
        let tmf_mesh = TMFMesh::read_from_obj_one(&mut file).unwrap().0;
        tmf_mesh.verify().unwrap();
    }
    #[test]
    #[cfg(feature = "obj_import")]
    fn rw_susan_obj() {
        init_test_env();
        let mut file = std::fs::File::open("testing/susan.obj").unwrap();
        let (tmf_mesh, _name) = TMFMesh::read_from_obj_one(&mut file).unwrap();
        tmf_mesh.verify().unwrap();
        //tmf_mesh.optimize();
        let _out = std::fs::File::create("target/susan.obj").unwrap();
    }
    #[test]
    #[cfg(feature = "obj_import")]
    fn save_susan_tmf() {
        init_test_env();
        let mut file = std::fs::File::open("testing/susan.obj").unwrap();
        let (tmf_mesh, name) = TMFMesh::read_from_obj_one(&mut file).unwrap();
        tmf_mesh.verify().unwrap();
        let mut out = std::fs::File::create("target/test_res/susan.tmf").unwrap();
        assert!(name == "Suzanne", "Name should be Suzanne but is {name}");
        let prec = TMFPrecisionInfo::default();
        tmf_mesh.write_tmf_one(&mut out, &prec, name).unwrap();
    }
    #[ignore]
    #[test]
    #[cfg(feature = "obj_import")]
    fn save_nefertiti_tmf() {
        init_test_env();
        let mut file = match std::fs::File::open("testing/Nefertiti.obj") {
            Ok(file) => file,
            Err(_) => return,
        };
        let (mut tmf_mesh, name) = TMFMesh::read_from_obj_one(&mut file).unwrap();
        tmf_mesh.verify().unwrap();
        tmf_mesh.optimize();
        let mut out = std::fs::File::create("target/test_res/Nefertiti.tmf").unwrap();
        assert!(
            name == "Nefertiti",
            "Name should be Nefertiti but is {name}"
        );
        println!(
            "uv_tris:{:?}",
            &tmf_mesh.get_uv_triangles().unwrap().iter().max().unwrap()
        );
        let prec = TMFPrecisionInfo::default();
        tmf_mesh.write_tmf_one(&mut out, &prec, name).unwrap();
        panic!();
    }
    #[test]
    fn analize_suzan() {
        let mut file = std::fs::File::open("target/test_res/optimized_susan.tmf").unwrap();
        let mut out = Vec::new();
        file.read_to_end(&mut out).unwrap();
        let r_mesh = crate::tmf::RUNTIME
            .block_on(tmf_importer::TMFImportContext::analize(
                &mut (&out as &[u8]),
            ))
            .unwrap();
        todo!();
    }
    #[test]
    #[cfg(feature = "obj_import")]
    fn rw_susan_tmf() {
        init_test_env();
        let mut file = std::fs::File::open("testing/susan.obj").unwrap();
        let (mut tmf_mesh, name) = TMFMesh::read_from_obj_one(&mut file).unwrap();
        tmf_mesh.verify().unwrap();
        tmf_mesh.optimize();
        assert!(name == "Suzanne", "Name should be Suzanne but is {name}");
        let prec = TMFPrecisionInfo::default();
        let mut out = Vec::new();
        {
            tmf_mesh.write_tmf_one(&mut out, &prec, name).unwrap();
        }
        let (r_mesh, name) = TMFMesh::read_tmf_one(&mut (&out as &[u8])).unwrap();
        assert!(name == "Suzanne", "Name should be Suzanne but is {name}");
        r_mesh.verify().unwrap();
        let mut index = 0;
        let mut should_fail = false;
        for (v_src, v_read) in std::iter::zip(
            tmf_mesh.get_vertex_triangles().unwrap(),
            r_mesh.get_vertex_triangles().unwrap(),
        ) {
            //assert_eq!(v_src,v_read,"Position in vts:{index}");
            if v_src != v_read {
                println!("Error at index {index}:{v_src} != {v_read}");
                should_fail = true;
            }
            index += 1;
        }
        if should_fail {
            panic!("Test errors");
        }
        let mut out = std::fs::File::create("target/test_res/susan_ftmf.obj").unwrap();
        r_mesh.write_obj_one(&mut out, &name).unwrap();
    }
    #[cfg(all(feature = "triangulation", feature = "obj_import"))]
    fn rw_cube_obj_not_triangulated() {
        init_test_env();
        let mut file = std::fs::File::open("testing/cube.obj").unwrap();
        let meshes = TMFMesh::read_from_obj(&mut file).unwrap();
        for (mesh, name) in &meshes {
            mesh.verify().unwrap();
        }
        let mut out = std::fs::File::create("target/test_res/cube_ftmf.obj").unwrap();
        TMFMesh::write_obj(&meshes, &mut out).unwrap();
    }
    #[test]
    #[cfg(feature = "obj_import")]
    fn load_multpile_meshes_obj() {
        init_test_env();
        let mut file = std::fs::File::open("testing/multiple.obj").unwrap();
        let meshes = TMFMesh::read_from_obj(&mut file).unwrap();
        for mesh_name in meshes {
            let (mesh, name) = mesh_name;
            mesh.verify().unwrap();
            let mut out = std::fs::File::create(&format!("target/test_res/{}.obj", name)).unwrap();
            mesh.write_obj_one(&mut out, &name).unwrap();
        }
    }
    #[test]
    #[cfg(feature = "obj_import")]
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
    /*
    #[ignore]
    #[test]
    #[cfg(feature = "obj_import")]
    fn read_multi_mtl_obj() {
        init_test_env();
        let mut file = std::fs::File::open("testing/multi_mtl.obj").unwrap();
        let tmf_mesh = TMFMesh::read_from_obj_one(&mut file).unwrap().0;
        tmf_mesh.verify().unwrap();
        todo!();
    }*/
}
