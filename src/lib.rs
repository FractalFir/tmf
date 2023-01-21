mod normals;
mod obj;
/// Module used to handle reads of data which is not bit aligned(for example, 3 or 17 bits). This is the module that allows for heavy compression used in this format.
#[doc(hidden)]
pub mod unaligned_rw;
mod uv;
mod vertices;
mod unaligned_lz;
// Unfinished
#[allow(dead_code)]
mod metadata;
const TMF_MAJOR: u16 = 0;
const TMF_MINOR: u16 = 1;
#[allow(non_camel_case_types)]
type float = f32;
type VertexType = (float, float, float);
type UvType = (float, float);
type FaceType = u32;
use metadata::FileMetadata;
use std::io::{Read, Write};

pub use crate::vertices::VertexPrecisionMode;
pub struct TMFPrecisionInfo {
    vertex_precision: VertexPrecisionMode,
}
impl Default for TMFPrecisionInfo {
    fn default() -> Self {
        TMFPrecisionInfo {
            vertex_precision: VertexPrecisionMode(0.1),
        }
    }
}
#[repr(u16)]
#[derive(Debug)]
enum SectionHeader {
    Invalid = 0,
    VertexSegment = 1,
    VertexFaceSegment = 2,
    NormalSegment = 3,
    NormalFaceSegment = 4,
    UvSegment = 5,
    UvFaceSegment = 6,
    MetadataSegment = 7,
    GroupInfoSegment = 8,
    FaceGroupInfo = 9,
}
impl SectionHeader {
    fn from_u16(input: u16) -> Self {
        match input {
            1 => Self::VertexSegment,
            2 => Self::VertexFaceSegment,
            3 => Self::NormalSegment,
            4 => Self::NormalFaceSegment,
            5 => Self::UvSegment,
            6 => Self::UvFaceSegment,
            _ => Self::Invalid,
        }
    }
}
use std::io::Result;
/// Representation of a TMF mesh. Can be loaded from disk, imported from diffrent format, saved to disk, and exported to a diffrent format, or created using special functions. Since it can be user generated it may be invalid and **must** be verified before being saved, otherwise a "garbage" mesh may be saved, an error may occur or a panic may occur.
pub struct TMFMesh {
    metadata: Option<FileMetadata>,
    normals: Option<Box<[VertexType]>>,
    normal_faces: Option<Box<[u32]>>,
    vertices: Option<Box<[VertexType]>>,
    vertex_faces: Option<Box<[u32]>>,
    uvs: Option<Box<[(f32, f32)]>>,
    uv_faces: Option<Box<[u32]>>,
    //groups: Option<Box<[String]>,Box<[u32]>>,
}
/// Enum representing the result of integrity check.
#[derive(Clone, Copy)]
pub enum TMFIntegrityStatus {
    /// Mesh has passed all verification steps
    Ok,
    /// No vertex array - mesh contains no points.
    VertexArrayMissing,
    /// Mesh contains info about vertices, but does not contain info about triangles they form.
    VertexFaceArrayMissing,
    /// Array contians index *0* which is outside vertex array length *1*
    VertexIndexOutsideVertexArray(u32, u32),
    /// Mesh contains no normal array
    NormalArrayMissing,
    /// Mesh contains no normal face array to set which trinagles have which normals from the normal array.
    NormalFaceArrayMissing,
    /// Index in traingle is outside the normal array.
    NormalIndexOutsideNormalArray,
    /// Mesh contains normals that are not normalized
    NormalNotNormalized,
    /// Mesh does not contain UV array.
    UVArrayMissing,
    /// Mesh contains uvs, but no uv face array to create triangles using those uvs.
    UVFaceArrayMissing,
}
impl TMFIntegrityStatus {
    /// Returns true if status is Ok.
    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Ok)
    }
    /// Returns true if mesh can't be safely saved to disk in current state(some action must be taken). Saving such a mesh would result in either a write error, garbage mesh saved, or a panic - depending on severity
    pub fn is_fatal(&self) -> bool {
        matches!(
            self,
            Self::VertexFaceArrayMissing
                | Self::NormalFaceArrayMissing
                | Self::UVFaceArrayMissing
                | Self::VertexIndexOutsideVertexArray(_, _)
                | Self::NormalIndexOutsideNormalArray
                | Self::NormalNotNormalized
        )
    }
    /// Returns status if fatal, otherwise Ok.
    pub fn only_fatal(&self) -> Self {
        if self.is_fatal() {
            *self
        } else {
            Self::Ok
        }
    }
    /// Checks if state is an error state (not Ok).
    pub fn is_err(&self) -> bool {
        !matches!(self, Self::Ok)
    }
    /// Works like unwrap on [`Result`](panics if not Ok)
    pub fn unwrap(&self) {
        match self {
            Self::Ok => (),
            _ => panic!("{self}"),
        }
    }
    /// Works like except on [`Result`](panics with *msg* if not Ok)
    pub fn except(&self, msg: &str) {
        match self {
            Self::Ok => (),
            _ => panic!("{msg}:{self}"),
        }
    }
}
impl std::fmt::Display for TMFIntegrityStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ok => write!(f, "Ok"),
            Self::VertexArrayMissing => write!(f, "VertexArrayMissing"),
            Self::VertexFaceArrayMissing => write!(f, "VertexFaceArrayMissing"),
            Self::VertexIndexOutsideVertexArray(index, size) => write!(
                f,
                "VertexIndexOutsideVertexArray{{index:{index},size:{size}}}"
            ),
            Self::NormalArrayMissing => write!(f, "NormalArrayMissing"),
            Self::NormalFaceArrayMissing => write!(f, "NormalFaceArrayMissing"),
            Self::NormalIndexOutsideNormalArray => write!(f, "NormalIndexOutsideNormalArray"),
            Self::NormalNotNormalized => write!(f, "NormalNotNormalized"),
            Self::UVArrayMissing => write!(f, "UVArrayMissing"),
            Self::UVFaceArrayMissing => write!(f, "UVFaceArrayMissing"),
        }
    }
}
fn slice_to_box<T: Sized + std::marker::Copy>(slice: &[T]) -> Box<[T]> {
    slice.into()
}
impl TMFMesh {
    /// Sets mesh vertex array and returns old vertex array if present. New mesh data is **not** checked during this function call, so to ensure mesh is valid call `verify` before saving.
    pub fn set_vertices(&mut self, vertices: &[VertexType]) -> Option<Box<[VertexType]>> {
        let mut vertices = Some(slice_to_box(vertices));
        std::mem::swap(&mut vertices, &mut self.vertices);
        vertices
    }
    /// Sets mesh normal array and returns old normal array if present. New mesh data is **not** checked during this function call, so to ensure mesh is valid call `verify` before saving.
    pub fn set_normals(&mut self, normals: &[VertexType]) -> Option<Box<[VertexType]>> {
        let mut normals = Some(slice_to_box(normals));
        std::mem::swap(&mut normals, &mut self.normals);
        normals
    }
    /// Sets mesh uv array and returns old uv array if present. New mesh daata is **not** checked during this function call, so to ensure mesh is valid call [`verify`] before saving.
    pub fn set_uvs(&mut self, uvs: &[UvType]) -> Option<Box<[UvType]>> {
        let mut uvs = Some(slice_to_box(uvs));
        std::mem::swap(&mut uvs, &mut self.uvs);
        uvs
    }
    /// Sets vertex face array to *faces* and returns old faces if present.
    pub fn set_vertex_faces(&mut self, faces: &[FaceType]) -> Option<Box<[FaceType]>> {
        let mut faces = Some(slice_to_box(faces));
        std::mem::swap(&mut faces, &mut self.vertex_faces);
        faces
    }
    /// Sets normal face array to *faces* and returns old faces if present.
    pub fn set_normal_faces(&mut self, faces: &[FaceType]) -> Option<Box<[FaceType]>> {
        let mut faces = Some(slice_to_box(faces));
        std::mem::swap(&mut faces, &mut self.normal_faces);
        faces
    }
    /// Sets uv face array to *faces* and returns old faces if present.
    pub fn set_uv_faces(&mut self, faces: &[FaceType]) -> Option<Box<[FaceType]>> {
        let mut faces = Some(slice_to_box(faces));
        std::mem::swap(&mut faces, &mut self.uv_faces);
        faces
    }
    /// Gets the vertices of this TMFMesh.
    pub fn get_vertices(&self) -> Option<&[VertexType]> {
        match &self.vertices {
            Some(vertices) => Some(vertices.as_ref()),
            None => None,
        }
    }
    /// Gets the normals of this TMFMesh.
    pub fn get_normals(&self) -> Option<&[VertexType]> {
        match &self.normals {
            Some(normals) => Some(normals.as_ref()),
            None => None,
        }
    }
    /// Gets the uv of this TMFMesh.
    pub fn get_uvs(&self) -> Option<&[UvType]> {
        match &self.uvs {
            Some(uvs) => Some(uvs.as_ref()),
            None => None,
        }
    }
    /// Gets the vertex face index array of this TMFMesh.
    pub fn get_vertex_faces(&self) -> Option<&[FaceType]> {
        match &self.vertex_faces {
            Some(vertex_faces) => Some(vertex_faces.as_ref()),
            None => None,
        }
    }
    /// Gets the normal face index array of this TMFMesh.
    pub fn get_normal_faces(&self) -> Option<&[FaceType]> {
        match &self.normal_faces {
            Some(normal_faces) => Some(normal_faces.as_ref()),
            None => None,
        }
    }
    /// Gets the uv face index array of this TMFMesh.
    pub fn get_uv_faces(&self) -> Option<&[FaceType]> {
        match &self.uv_faces {
            Some(uv_faces) => Some(uv_faces.as_ref()),
            None => None,
        }
    }
    /// Normalizes normal array, if present
    /*
    pub fn normalize(&mut self){
        use crate::normals::normalize_arr;
        match &self.normals{
            Some(mut normals) => normalize_arr(&mut normals),
            None=>(),
        }

    }
    */
    /// Checks if mesh is valid and can be saved.
    pub fn verify(&self) -> TMFIntegrityStatus {
        // Check that vertex and vertex face array are present
        match &self.vertices {
            Some(vertices) => match &self.vertex_faces {
                Some(vertex_faces) => {
                    for index in vertex_faces.iter() {
                        if *index >= vertices.len() as u32 {
                            //Vertex index outside vertex array!
                            return TMFIntegrityStatus::VertexIndexOutsideVertexArray(
                                *index,
                                vertices.len() as u32,
                            );
                        }
                    }
                }
                None => return TMFIntegrityStatus::VertexFaceArrayMissing,
            },
            None => return TMFIntegrityStatus::VertexArrayMissing,
        }
        // Check if only normals xor normal faces present, if so invalid.
        if !(self.normals.is_some() ^ self.normal_faces.is_some()) {
            // Normals without faces or faces without normals
            if self.normals.is_none() {
                return TMFIntegrityStatus::NormalArrayMissing;
            }
            if self.normal_faces.is_none() {
                return TMFIntegrityStatus::NormalFaceArrayMissing;
            }
        }
        // If normals and normal faces present, verify them.
        else if self.normals.is_some() && self.normal_faces.is_some() {
            let normals = self.normals.as_ref().unwrap();
            let normal_faces = self.normal_faces.as_ref().unwrap();
            for index in normal_faces.iter() {
                if *index >= normals.len() as u32 {
                    //Normal index outside normal array
                    return TMFIntegrityStatus::NormalIndexOutsideNormalArray;
                }
            }
            for normal in normals.iter() {
                let mag = normals::magnitude(*normal);
                if (1.0 - mag) > 0.001 {
                    //Not normalised normals
                    return TMFIntegrityStatus::NormalNotNormalized;
                }
            }
        }
        // UV and UV Faces most be either both missing or both present, else invalid
        if self.uvs.is_some() ^ self.uv_faces.is_some() {
            if self.uvs.is_none() {
                return TMFIntegrityStatus::UVArrayMissing;
            }
            if self.uv_faces.is_none() {
                return TMFIntegrityStatus::UVFaceArrayMissing;
            }
        }
        return TMFIntegrityStatus::Ok;
    }
    /// Reads tmf mesh from a .obj file in *reader*
    pub fn read_from_obj<R: Read>(reader: &mut R) -> Result<Self> {
        obj::read_from_obj(reader)
    }
    /// Writes this TMF  mesh to a .obj file.
    pub fn write_obj<W: Write>(&self, w: &mut W) -> Result<()> {
        obj::write_obj(self, w)
    }
    /// Writes this TMF Mesh to *w*.
    pub fn write_tmf<W: Write>(&self, w: &mut W, p_info: &TMFPrecisionInfo) -> Result<()> {
        let mut curr_segment_data = Vec::with_capacity(0x100);
        w.write_all(b"TMF")?;
        w.write_all(&TMF_MAJOR.to_le_bytes())?;
        w.write_all(&TMF_MINOR.to_le_bytes())?;
        match &self.metadata {
            Some(metadata) => todo!("Saving metadata is not yet supported!"),
            None => (),
        }
        //Calculate shortest edge, or if no edges present, 1.0
        let shortest_edge = match &self.vertex_faces {
            Some(vertex_faces) => {
                let vertices = match &self.vertices{
                    Some(vertices)=>vertices,
                    None=>return Err(std::io::Error::new(std::io::ErrorKind::Other,"Saving a mesh with face normal index array without normal array is an error.")),
                };
                fn dst(a: (f32, f32, f32), b: (f32, f32, f32)) -> f32 {
                    let dx = a.0 - b.0;
                    let dy = a.1 - b.1;
                    let dz = a.2 - b.2;
                    (dx * dx + dy * dy + dz * dz).sqrt()
                }
                let mut shortest_edge = f32::INFINITY;
                for i in 0..(vertex_faces.len() / 3) {
                    let d1 = dst(
                        vertices[vertex_faces[i * 3] as usize],
                        vertices[vertex_faces[i * 3 + 1] as usize],
                    );
                    let d2 = dst(
                        vertices[vertex_faces[i * 3 + 1] as usize],
                        vertices[vertex_faces[i * 3 + 2] as usize],
                    );
                    let d3 = dst(
                        vertices[vertex_faces[i * 3 + 2] as usize],
                        vertices[vertex_faces[i * 3] as usize],
                    );
                    shortest_edge = shortest_edge.min(d1.min(d2.min(d3)));
                }
                shortest_edge
            }
            None => 1.0,
        };
        // Save vertices
        match &self.vertices {
            Some(vertices) => {
                use crate::vertices::save_tmf_vertices;
                save_tmf_vertices(
                    vertices,
                    p_info.vertex_precision,
                    &mut curr_segment_data,
                    shortest_edge,
                )?;
                w.write_all(&(SectionHeader::VertexSegment as u16).to_le_bytes())?;
                w.write_all(&(curr_segment_data.len() as u32).to_le_bytes())?;
                w.write_all(&curr_segment_data)?;
                curr_segment_data.clear();
            }
            None => (),
        }
        // Save vertex faces
        match &self.vertex_faces {
            Some(vertex_faces) => {
                use crate::vertices::save_faces;
                //If saving vertex faces, vertices must be present, so unwrap can't fail
                let v_count = self.vertices.as_ref().unwrap().len();
                save_faces(vertex_faces, v_count, &mut curr_segment_data)?;
                w.write_all(&(SectionHeader::VertexFaceSegment as u16).to_le_bytes())?;
                w.write_all(&(curr_segment_data.len() as u32).to_le_bytes())?;
                w.write_all(&curr_segment_data)?;
                curr_segment_data.clear();
            }
            None => (),
        };
        // Save Normals
        match &self.normals {
            Some(normals) => {
                use crate::normals::*;
                save_normal_array(
                    normals,
                    &mut curr_segment_data,
                    NormalPrecisionMode::from_deg_dev(0.01),
                )?;
                w.write_all(&(SectionHeader::NormalSegment as u16).to_le_bytes())?;
                w.write_all(&(curr_segment_data.len() as u32).to_le_bytes())?;
                w.write_all(&curr_segment_data)?;
                curr_segment_data.clear();
            }
            None => (),
        };
        // Save normal faces
        match &self.normal_faces {
            Some(normal_faces) => {
                use crate::vertices::save_faces;
                //If saving normal faces, normals must be present, so unwrap can't fail
                let n_count = self.normals.as_ref().unwrap().len();
                save_faces(normal_faces, n_count, &mut curr_segment_data)?;
                w.write_all(&(SectionHeader::NormalFaceSegment as u16).to_le_bytes())?;
                w.write_all(&(curr_segment_data.len() as u32).to_le_bytes())?;
                w.write_all(&curr_segment_data)?;
                curr_segment_data.clear();
            }
            None => (),
        };
        match &self.uvs {
            Some(uvs) => {
                uv::save_uvs(uvs, &mut curr_segment_data, 0.001)?;
                w.write_all(&(SectionHeader::UvSegment as u16).to_le_bytes())?;
                w.write_all(&(curr_segment_data.len() as u32).to_le_bytes())?;
                w.write_all(&curr_segment_data)?;
                curr_segment_data.clear();
            }
            None => (),
        }
        // Save uv faces
        match &self.uv_faces {
            Some(uv_faces) => {
                use crate::vertices::save_faces;
                //If saving uv faces, uvs must be present, so unwrap can't fail
                let uv_count = self.uvs.as_ref().unwrap().len();
                save_faces(uv_faces, uv_count, &mut curr_segment_data)?;
                w.write_all(&(SectionHeader::UvFaceSegment as u16).to_le_bytes())?;
                w.write_all(&(curr_segment_data.len() as u32).to_le_bytes())?;
                w.write_all(&curr_segment_data)?;
                curr_segment_data.clear();
            }
            None => (),
        };
        Ok(())
    }
    /// Creates an empty TMF Mesh.
    pub fn empty() -> Self {
        Self {
            metadata: None,
            normal_faces: None,
            normals: None,
            uv_faces: None,
            uvs: None,
            vertex_faces: None,
            vertices: None,
        }
    }
    /// Reads a mesh from a .tmf file.
    pub fn read_tmf<R: Read>(reader: &mut R) -> Result<Self> {
        let mut magic = [0; 3];
        reader.read_exact(&mut magic)?;
        if magic != *b"TMF" {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Not a TMF file",
            ));
        }
        // Not used ATM, but can be used for compatiblity in the future.
        let _major = {
            let mut tmp = [0; 2];
            reader.read_exact(&mut tmp)?;
            u16::from_le_bytes(tmp)
        };
        // Not used ATM, but can be used for compatiblity in the future.
        let _minor = {
            let mut tmp = [0; 2];
            reader.read_exact(&mut tmp)?;
            u16::from_le_bytes(tmp)
        };
        fn read_u16<R: Read>(r: &mut R) -> Result<u16> {
            let mut tmp = [0; 2];
            r.read_exact(&mut tmp)?;
            Ok(u16::from_le_bytes(tmp))
        }
        let mut res = Self::empty();
        while let Ok(header) = read_u16(reader) {
            let header = SectionHeader::from_u16(header);
            let data_length = {
                let mut tmp = [0; 4];
                reader.read_exact(&mut tmp)?;
                u32::from_le_bytes(tmp)
            };
            let mut data = vec![0; data_length as usize];
            reader.read_exact(&mut data)?;
            match header {
                SectionHeader::VertexSegment => {
                    use crate::vertices::read_tmf_vertices;
                    match &res.vertices {
                        Some(_) => {
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                "Only one vertex array can be present in a model.",
                            ))
                        }
                        None => res.vertices = Some(read_tmf_vertices(&mut (&data as &[u8]))?),
                    }
                }
                SectionHeader::NormalSegment => {
                    use crate::normals::read_normal_array;
                    match &res.normals {
                        Some(_) => {
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                "Only one normal array can be present in a model.",
                            ))
                        }
                        None => res.normals = Some(read_normal_array(&mut (&data as &[u8]))?),
                    }
                }
                SectionHeader::UvSegment => {
                    use crate::uv::read_uvs;
                    match &res.uvs {
                        Some(_) => {
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                "Only one uv array can be present in a model.",
                            ))
                        }
                        None => res.uvs = Some(read_uvs(&mut (&data as &[u8]))?),
                    }
                }
                SectionHeader::VertexFaceSegment => {
                    use vertices::read_faces;
                    match &res.vertex_faces {
                        Some(_) => return Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "Only one vertex index array(face array) can be present in a model.",
                        )),
                        None => res.vertex_faces = Some(read_faces(&mut (&data as &[u8]))?),
                    }
                }
                SectionHeader::NormalFaceSegment => {
                    use vertices::read_faces;
                    match &res.normal_faces {
                        Some(_) => return Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "Only one normal index array(face array) can be present in a model.",
                        )),
                        None => res.normal_faces = Some(read_faces(&mut (&data as &[u8]))?),
                    }
                }
                SectionHeader::UvFaceSegment => {
                    use vertices::read_faces;
                    match &res.uv_faces {
                        Some(_) => {
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                "Only one uv index array(face array) can be present in a model.",
                            ))
                        }
                        None => res.uv_faces = Some(read_faces(&mut (&data as &[u8]))?),
                    }
                }
                _ => (), //Unknown header, ignoring
            }
        }
        //todo!();
        Ok(res)
    }
}
#[cfg(test)]
mod testing {
    use super::*;
    #[test]
    fn read_susan_obj() {
        let mut file = std::fs::File::open("testing/susan.obj").unwrap();
        let tmf_mesh = TMFMesh::read_from_obj(&mut file).unwrap();
        tmf_mesh.verify().unwrap();
    }
    #[test]
    fn rw_susan_obj() {
        let mut file = std::fs::File::open("testing/susan.obj").unwrap();
        let tmf_mesh = TMFMesh::read_from_obj(&mut file).unwrap();
        tmf_mesh.verify().unwrap();
        let mut out = std::fs::File::create("target/susan.obj").unwrap();
        tmf_mesh.write_obj(&mut out).unwrap();
    }
    #[test]
    fn save_susan_tmf() {
        let mut file = std::fs::File::open("testing/susan.obj").unwrap();
        let tmf_mesh = TMFMesh::read_from_obj(&mut file).unwrap();
        tmf_mesh.verify().unwrap();
        let mut out = std::fs::File::create("target/susan.tmf").unwrap();
        tmf_mesh
            .write_tmf(&mut out, &TMFPrecisionInfo::default())
            .unwrap();
    }
    #[test]
    fn rw_susan_tmf() {
        let mut file = std::fs::File::open("testing/susan.obj").unwrap();
        let tmf_mesh = TMFMesh::read_from_obj(&mut file).unwrap();
        tmf_mesh.verify().unwrap();
        let mut out = Vec::new();
        {
            tmf_mesh
                .write_tmf(&mut out, &TMFPrecisionInfo::default())
                .unwrap();
        }
        let r_mesh = TMFMesh::read_tmf(&mut (&out as &[u8])).unwrap();
        r_mesh.verify().unwrap();
        let mut out = std::fs::File::create("target/susan_ftmf.obj").unwrap();
        r_mesh.write_obj(&mut out).unwrap();
    }
    #[test]
    #[should_panic]
    fn rw_cube_obj_not_triangulated() {
        let mut file = std::fs::File::open("testing/cube.obj").unwrap();
        let tmf_mesh = TMFMesh::read_from_obj(&mut file).unwrap();
        tmf_mesh.verify().unwrap();
        let mut out = std::fs::File::create("target/cube.obj").unwrap();
        tmf_mesh.write_obj(&mut out).unwrap();
    }
}
