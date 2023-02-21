#![warn(missing_docs)]
#![warn(rustdoc::missing_doc_code_examples)]
//! **tmf** is a crate used to save and read 3D models saved in *.tmf* format. This format is focused on 2 things:
//! 1. Reducing size of the saved file as much as possible, without reducing visual quality
//! 2. Loading models as fast as possible(without sacrificing model size reduction)
//! This means that while saving a model may take a slightly longer time (2-4x loading), models can be loaded at considerable speed(loading a model with around 40 000 points takes 1.6 ms)
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
#[doc(inline)]
pub use crate::normals::NormalPrecisionMode;
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
    pub uv_precision: UvPrecisionMode,
    /// Do additional normal pruning before saving (has considerable performance impact if model has many vertices
    pub prune_normals: bool,
}
impl Default for TMFPrecisionInfo {
    /// Returns the default, middle-ground settings for saving meshes. Should be indistinguishable by human eye, but the LOD may be not enough for some rare cases (eg. procedural generation).
    fn default() -> Self {
        TMFPrecisionInfo {
            vertex_precision: VertexPrecisionMode::default(),
            normal_precision: NormalPrecisionMode::default(),
            uv_precision: UvPrecisionMode::default(),
            prune_normals: true,
        }
    }
}
use std::io::Result;
/// Representation of a TMF mesh. Can be loaded from disk, imported from diffrent format, saved to disk, and exported to a diffrent format, or created using special functions. Any mesh created at run time *should* but does not *have to* be checked before saving with [`Self::verify`] call. If the mesh is known to be OK before saving this step can be skipped(even tough it is still advised).
#[derive(Clone)]
pub struct TMFMesh {
    normals: Option<Box<[Vector3]>>,
    normal_triangles: Option<Box<[IndexType]>>,
    vertices: Option<Box<[Vector3]>>,
    vertex_triangles: Option<Box<[IndexType]>>,
    uvs: Option<Box<[Vector2]>>,
    uv_triangles: Option<Box<[IndexType]>>,
    materials: Option<MaterialInfo>,
}
impl Default for TMFMesh {
    /// Creates default, empty [`TMFMesh`]. Equivalent to [`TMFMesh::empty`] call.
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
        if self.normal_triangles.is_some() {
            count += 1
        };
        if self.vertices.is_some() {
            count += 1
        };
        if self.vertex_triangles.is_some() {
            count += 1
        };
        if self.uvs.is_some() {
            count += 1
        };
        if self.uv_triangles.is_some() {
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
    /// Sets vertex index array to *triangles* and returns old triangles if present.
    ///```
    /// # use tmf::TMFMesh;
    /// # let mut mesh = TMFMesh::empty();
    /// # let triangles = [0,1,2,3,2,1];
    /// mesh.set_vertex_triangles(&triangles);
    ///```
    pub fn set_vertex_triangles(&mut self, triangles: &[IndexType]) -> Option<Box<[IndexType]>> {
        let mut triangles = Some(slice_to_box(triangles));
        std::mem::swap(&mut triangles, &mut self.vertex_triangles);
        triangles
    }
    /// Sets normal index array to *triangles* and returns old triangles if present.
    ///```
    /// # use tmf::TMFMesh;
    /// # let mut mesh = TMFMesh::empty();
    /// # let triangles = [0,1,2,3,2,1];
    /// mesh.set_normal_triangles(&triangles);
    ///```
    pub fn set_normal_triangles(&mut self, triangles: &[IndexType]) -> Option<Box<[IndexType]>> {
        let mut triangles = Some(slice_to_box(triangles));
        std::mem::swap(&mut triangles, &mut self.normal_triangles);
        triangles
    }
    /// Sets uv index array to *triangles* and returns old triangles if present.
    ///```
    /// # use tmf::TMFMesh;
    /// # let mut mesh = TMFMesh::empty();
    /// # let triangles = [0,1,2,3,2,1];
    /// mesh.set_uv_triangles(&triangles);
    ///```
    pub fn set_uv_triangles(&mut self, triangles: &[IndexType]) -> Option<Box<[IndexType]>> {
        let mut triangles = Some(slice_to_box(triangles));
        std::mem::swap(&mut triangles, &mut self.uv_triangles);
        triangles
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
    /// Gets the vertex triangle index array of this [`TMFMesh`].
    ///```
    /// # use tmf::TMFMesh;
    /// # let mesh = TMFMesh::empty();
    /// let vertex_triangles = mesh.get_vertex_triangles();
    ///```
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
    pub fn get_normal_triangles(&self) -> Option<&[IndexType]> {
        match &self.normal_triangles {
            Some(normal_triangles) => Some(normal_triangles.as_ref()),
            None => None,
        }
    }
    /// Gets the uv triangle index array of this TMFMesh.
    ///```
    /// # use tmf::TMFMesh;
    /// # let mesh = TMFMesh::empty();
    /// let uv_triangles = mesh.get_uv_triangles();
    ///```
    pub fn get_uv_triangles(&self) -> Option<&[IndexType]> {
        match &self.uv_triangles {
            Some(uv_triangles) => Some(uv_triangles.as_ref()),
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
    /// # mesh.set_vertices(&vertices);
    /// # mesh.set_vertex_triangles(&vertex_triangles);
    /// let vert_buff = mesh.get_vertex_buffer().expect("Could not create the array of points creating triangles!");
    /// // The same number of triangles created by points and triangles created by indices
    /// assert!(vert_buff.len() == vertex_triangles.len());
    ///```
    pub fn get_vertex_buffer(&self) -> Option<Box<[Vector3]>> {
        let vertices = self.get_vertices()?;
        let triangles = self.get_vertex_triangles()?;
        let mut vertex_buffer = Vec::with_capacity(triangles.len());
        for index in triangles {
            match vertices.get(*index as usize) {
                Some(vertex) => vertex_buffer.push(*vertex),
                None => panic!("Invalid TMFMesh: vertex index outside vertex array!"),
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
    /// # mesh.set_normals(&normals);
    /// # mesh.normalize();
    /// # mesh.set_normal_triangles(&normal_triangles);
    /// let normal_buff = mesh.get_normal_buffer().expect("Could not create the array of normals creating triangles!");
    /// // The same number of triangles created by points and triangles created by indices
    /// assert!(normal_buff.len() == normal_triangles.len());
    ///```
    pub fn get_normal_buffer(&self) -> Option<Box<[Vector3]>> {
        let normals = self.get_normals()?;
        let triangles = self.get_normal_triangles()?;
        let mut normals_buffer = Vec::with_capacity(triangles.len());
        for index in triangles {
            match normals.get(*index as usize) {
                Some(normal) => normals_buffer.push(*normal),
                None => panic!("Invalid TMFMesh: normal index outside vertex array!"),
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
    /// # mesh.set_uvs(&uvs);
    /// # mesh.set_uv_triangles(&uv_triangles);
    /// let uv_buff = mesh.get_uv_buffer().expect("Could not create the array of uvs creating triangles!");
    /// // The same number of triangles created by points and triangles created by indices
    /// assert!(uv_buff.len() == uv_triangles.len());
    ///```
    pub fn get_uv_buffer(&self) -> Option<Box<[Vector2]>> {
        let uvs = self.get_uvs()?;
        let triangles = self.get_uv_triangles()?;
        let mut uv_buffer = Vec::with_capacity(triangles.len());
        for index in triangles {
            match uvs.get(*index as usize) {
                Some(uv) => uv_buffer.push(*uv),
                None => panic!("Invalid TMFMesh: uv index outside vertex array!"),
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
    /// mesh.set_normals(&normals);
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
        match normals {
            Some(normals) => normalize_arr(normals),
            None => (),
        }
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
    /// Does not support not triangulated models ATM
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
    /// Does not support not triangulated models ATM
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
    /// # use tmf::TMFMesh;
    /// # let mesh = TMFMesh::empty();
    /// # let out_path ="target/test_res/doc.obj";
    /// let mut obj_out = File::create(out_path).expect("Could not create obj out file!");
    /// mesh.write_obj_one(&mut obj_out,"mesh name").expect("Could not write the .obj file!");
    /// ```
    pub fn write_obj_one<W: Write>(&self, w: &mut W, name: &str) -> Result<()> {
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
    pub fn write_obj<W: Write, S: std::borrow::Borrow<str>>(
        meshes: &[(TMFMesh, S)],
        w: &mut W,
    ) -> Result<()> {
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
    pub fn write_tmf_one<W: Write, S: std::borrow::Borrow<str>>(
        &self,
        w: &mut W,
        p_info: &TMFPrecisionInfo,
        name: S,
    ) -> Result<()> {
        tmf::write_tmf_header(w, 1)?;
        tmf::write_mesh(self, w, p_info, name.borrow())
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
    pub fn write_tmf<W: Write, S: std::borrow::Borrow<str>>(
        meshes_names: &[(Self, S)],
        w: &mut W,
        p_info: &TMFPrecisionInfo,
    ) -> Result<()> {
        tmf::write(meshes_names, w, p_info)
    }
    /// Creates an empty TMF Mesh(mesh with no data). Equivalent to [`TMFMesh::default`].
    /// ```
    /// # use tmf::TMFMesh;
    /// // Creates an empty mesh with no data
    /// let mut mesh = TMFMesh::empty();
    /// ```
    pub fn empty() -> Self {
        Self {
            normal_triangles: None,
            normals: None,
            uv_triangles: None,
            uvs: None,
            vertex_triangles: None,
            vertices: None,
            materials: None,
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
    pub fn read_tmf<R: Read>(reader: &mut R) -> Result<Vec<(Self, String)>> {
        Ok(tmf::read(reader)?)
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
        let prec = TMFPrecisionInfo::default();
        tmf_mesh.write_tmf_one(&mut out, &prec, name).unwrap();
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
                .write_tmf_one(&mut out, &TMFPrecisionInfo::default(), name)
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
    #[ignore]
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
        let (tmf_mesh, name) = TMFMesh::read_from_obj_one(&mut file).unwrap();
        tmf_mesh.verify().unwrap();
        let mut out = Vec::new();
        {
            tmf_mesh
                .write_tmf_one(&mut out, &TMFPrecisionInfo::default(), name)
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
