/// Module used to handle reads of data which is not bit aligned(for example, 3 or 17 bits). This is the module that allows for heavy compression used in this format.
pub mod unaligned_rw;
pub mod normals;
pub mod vertices;
mod uv;
const TMF_MAJOR:u16 = 0;
const TMF_MINOR:u16 = 1;
use std::io::{Write,Read,BufReader};
#[repr(u16)]#[derive(Debug)]
enum SectionHeader{
    Invalid = 0,
    VertexSegment = 1,
    VertexFaceSegment = 2,
    NormalSegment = 3,
    NormalFaceSegment = 4,
    UvSegment = 5,
    UvFaceSegement = 6,
}
struct FileMetadata{
    author:String,
}
use std::io::Result;
struct TMFMesh{
    metadata:Option<FileMetadata>,
    normals:Option<Box<[(f32,f32,f32)]>>,
    normal_faces:Option<Box<[u32]>>,
    vertices:Option<Box<[(f32,f32,f32)]>>,
    vertex_faces:Option<Box<[u32]>>,
    uvs:Option<Box<[(f32,f32)]>>,
    uv_faces:Option<Box<[u32]>>,
}
pub enum TMFIntegrityStatus{
    Ok,
    VertexArrayMissing,
    VertexFaceArrayMissing,
    VertexIndexOutsideVertexArray(u32,u32),
    NormalArrayMissing,
    NormalFaceArrayMissing,
    NormalIndexOutsideNormalArray,
    NormalNotNormalized,
    UVArrayMissing,
    UVFaceArrayMissing,
}
impl TMFIntegrityStatus{
    pub fn is_ok(&self)->bool{
        match self{
            Self::Ok=>true,
            _=>false,
        }
    }
    pub fn is_err(&self)->bool{
        match self{
            Self::Ok=>false,
            _=>true,
        }
    }
    pub fn unwrap(&self){
        match self{
            Self::Ok=>(),
            _=>panic!("{self}"),
        }
    }
    pub fn except(&self,msg:&str){
        match self{
            Self::Ok=>(),
            _=>panic!("{msg}:{self}"),
        }
    }
}
impl std::fmt::Display for TMFIntegrityStatus{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result{
        match self{
            Self::Ok=>                                          write!(f,"Ok"),
            Self::VertexArrayMissing=>                          write!(f,"VertexArrayMissing"),
            Self::VertexFaceArrayMissing=>                      write!(f,"VertexFaceArrayMissing"),
            Self::VertexIndexOutsideVertexArray(index,size)=>   write!(f,"VertexIndexOutsideVertexArray{{index:{index},size:{size}}}"),
            Self::NormalArrayMissing=>                          write!(f,"NormalArrayMissing"),
            Self::NormalFaceArrayMissing=>                      write!(f,"NormalFaceArrayMissing"),
            Self::NormalIndexOutsideNormalArray=>               write!(f,"NormalIndexOutsideNormalArray"),
            Self::NormalNotNormalized=>                         write!(f,"NormalNotNormalized"),
            Self::UVArrayMissing=>                              write!(f,"UVArrayMissing"),
            Self::UVFaceArrayMissing=>                          write!(f,"UVFaceArrayMissing"),
        }
    }
}
impl TMFMesh{
    pub fn verify(&self)->TMFIntegrityStatus{
        // Check that vertex and vertex face array are present
        match &self.vertices{
            Some(vertices)=>match &self.vertex_faces{
                Some(vertex_faces)=>{
                    for index in vertex_faces.iter(){
                        if *index >= vertices.len() as u32{
                            //Vertex index outside vertex array!
                            return TMFIntegrityStatus::VertexIndexOutsideVertexArray(*index,vertices.len() as u32);
                        }
                    }
                },
                None=>return TMFIntegrityStatus::VertexFaceArrayMissing,
            },
            None=>return TMFIntegrityStatus::VertexArrayMissing,
        }
        // Check if only normals xor normal faces present, if so invalid.
        if !(self.normals.is_some()^self.normal_faces.is_some()){
            // Normals without faces or faces without normals
            if self.normals.is_none(){
                return TMFIntegrityStatus::NormalArrayMissing;
            }
            if self.normal_faces.is_none(){
                return TMFIntegrityStatus::NormalFaceArrayMissing;
            }
        }
        // If normals and normal faces present, verify them.
        else if self.normals.is_some()&&self.normal_faces.is_some(){
            let normals = self.normals.as_ref().unwrap();
            let normal_faces = self.normal_faces.as_ref().unwrap();
            for index in normal_faces.iter(){
                if *index >= normals.len() as u32{
                    //Normal index outside normal array
                    return TMFIntegrityStatus::NormalIndexOutsideNormalArray;
                }
            }
            for normal in normals.iter(){
                let mag = normals::magnitude(*normal);
                if mag < 0.999 || mag > 1.001{
                    //Not normalised normals
                    return TMFIntegrityStatus::NormalNotNormalized;
                }
            }
        }
        // UV and UV Faces most be either both missing or both present, else invalid
        if self.uvs.is_some()^self.uv_faces.is_some(){
            if self.uvs.is_none(){
                return TMFIntegrityStatus::UVArrayMissing;
            }
            if self.uv_faces.is_none(){
                return TMFIntegrityStatus::UVFaceArrayMissing;
            }
        }
        return TMFIntegrityStatus::Ok;
    }
    fn match_split<'a>(split:Option<&'a str>)->Result<&'a str>{
        match split{
            Some(beg)=>Ok(beg),
            None=>Err(std::io::Error::new(std::io::ErrorKind::Other,"Invalid .obj line")),
        }
    }
    fn parse_f32(float:&str)->Result<f32>{
        match float.parse::<f32>(){
            Ok(float)=>Ok(float),
            Err(err)=>Err(std::io::Error::new(std::io::ErrorKind::Other,err.to_string())),
        }
    }
    fn parse_u32(uint:&str)->Result<u32>{
        match uint.parse::<u32>(){
            Ok(uint)=>Ok(uint),
            Err(err)=>Err(std::io::Error::new(std::io::ErrorKind::Other,err.to_string())),
        }
    }
    pub fn read_from_obj<R:Read>(reader:&mut R)->Result<Self>{
        use std::io::BufRead;
        let reader = BufReader::new(reader);
        let mut vertices      = Vec::with_capacity(0x100);
        let mut normals       = Vec::with_capacity(0x100);
        let mut uvs           = Vec::with_capacity(0x100);
        let mut vertex_faces  = Vec::with_capacity(0x100);
        let mut normal_faces  = Vec::with_capacity(0x100);
        let mut uv_faces      = Vec::with_capacity(0x100);
        for line in reader.lines(){
            let line = line?; 
            let mut split = line.split(&[' ','/']);
            let beg = Self::match_split(split.next())?;
            match beg{
                "v"=>{
                    let (x,y,z) = (Self::match_split(split.next())?,Self::match_split(split.next())?,Self::match_split(split.next())?);
                    let vertex = (Self::parse_f32(x)?,Self::parse_f32(y)?,Self::parse_f32(z)?);
                    vertices.push(vertex);
                },
                "vn"=>{
                    let (x,y,z) = (Self::match_split(split.next())?,Self::match_split(split.next())?,Self::match_split(split.next())?);
                    let normal = (Self::parse_f32(x)?,Self::parse_f32(y)?,Self::parse_f32(z)?);
                    normals.push(normal);
                },
                "vt"=>{
                    let (x,y) = (Self::match_split(split.next())?,Self::match_split(split.next())?);
                    let uv = (Self::parse_f32(x)?,Self::parse_f32(y)?);
                    uvs.push(uv);
                },
                "f"=>{
                     let (v0,vt0,vn0,v1,vt1,vn1,v2,vt2,vn2) = (
                     Self::match_split(split.next())?,Self::match_split(split.next())?,Self::match_split(split.next())?,
                     Self::match_split(split.next())?,Self::match_split(split.next())?,Self::match_split(split.next())?,
                     Self::match_split(split.next())?,Self::match_split(split.next())?,Self::match_split(split.next())?
                     );
                     if split.next().is_some(){
                        return Err(std::io::Error::new(std::io::ErrorKind::Other,"OBJ reader supports only triangulated meshes ATM."));
                     }
                     vertex_faces.push(Self::parse_u32(v0)?  - 1);
                     vertex_faces.push(Self::parse_u32(v1)?  - 1);
                     vertex_faces.push(Self::parse_u32(v2)?  - 1);
                     normal_faces.push(Self::parse_u32(vn0)? - 1);
                     normal_faces.push(Self::parse_u32(vn1)? - 1);
                     normal_faces.push(Self::parse_u32(vn2)? - 1);
                     uv_faces.push(Self::parse_u32(vt0)?     - 1);
                     uv_faces.push(Self::parse_u32(vt1)?     - 1);
                     uv_faces.push(Self::parse_u32(vt2)?     - 1);
                }
                "#"=>continue,
                "mtllib"=>continue,//TODO:use material info
                "o"=>continue,//TODO:use object info
                "s"=>continue,//TODO:use smoothness  info
                _=>todo!("{}",line),
            }
        }
        Ok(Self{
            metadata:None,
            vertices:Some(vertices.into()),
            vertex_faces:Some(vertex_faces.into()),
            normals:Some(normals.into()),
            normal_faces:Some(normal_faces.into()),
            uvs:Some(uvs.into()),
            uv_faces:Some(uv_faces.into()),
        })
    }
    fn write_obj<W:Write>(&self,w:&mut W)->Result<()>{
        match &self.vertices{
            None=>(),
            Some(vertices)=>{
                for vertex in vertices.into_iter(){
                    write!(w,"v {} {} {}\n",vertex.0,vertex.1,vertex.2)?;
                }
            }
        }
        match &self.normals{
            None=>(),
            Some(normals)=>{
                for normal in normals.into_iter(){
                    write!(w,"vn {} {} {}\n",normal.0,normal.1,normal.2)?;
                }
            }
        }
        match &self.uvs{
            None=>(),
            Some(uvs)=>{
                for uv in uvs.into_iter(){
                    write!(w,"vt {} {}\n",uv.0,uv.1)?;
                }
            }
        }
        match &self.vertex_faces{
            None=>return Err(std::io::Error::new(std::io::ErrorKind::Other,"Vertex faces array must be present when saving to .obj file")),
            Some(vertex_faces)=>{
                let normal_faces = match &self.normal_faces{
                    Some(normal_faces)=>normal_faces,
                    None=>return Err(std::io::Error::new(std::io::ErrorKind::Other,"Normal face arrays must be present when saving to .obj file")),
                };
                let uv_faces = match &self.uv_faces{
                    Some(uv_faces)=>uv_faces,
                    None=>return Err(std::io::Error::new(std::io::ErrorKind::Other,"UV face arrays must be present when saving to .obj file")),
                };
                if vertex_faces.len() != normal_faces.len() || vertex_faces.len() != uv_faces.len(){
                    return Err(std::io::Error::new(std::io::ErrorKind::Other,"Face Array size mismatch"));
                }
                for i in 0..vertex_faces.len(){
                    if i%3 == 0{write!(w,"f ")?};
                    write!(w,"{}/{}/{} ",vertex_faces[i] + 1,uv_faces[i] + 1,normal_faces[i] + 1)?;
                    if i%3 == 2{write!(w,"\n")?};
                }
            }
        }
        Ok(())
    }
    pub fn write_tmf<W:Write>(&self,w:&mut W)->Result<()>{
        let mut curr_segment_data = Vec::with_capacity(0x100);
        w.write_all(b"TMF")?;
        w.write_all(&TMF_MAJOR.to_le_bytes())?;
        w.write_all(&TMF_MINOR.to_le_bytes())?;
        match &self.metadata{
            Some(metadata)=>todo!("Saving metadata is not yet supported!"),
            None=>(),
        }
        //Calculate shortest edge, or if no edges present, 1.0
        let shortest_edge = match &self.vertex_faces{
            Some(vertex_faces)=>{
                let vertices = match &self.vertices{
                    Some(vertices)=>vertices,
                    None=>return Err(std::io::Error::new(std::io::ErrorKind::Other,"Saving a mesh with face normal index array without normal array is an error.")),
                };
                fn dst(a:(f32,f32,f32),b:(f32,f32,f32))->f32{
                    let dx = a.0 - b.0;
                    let dy = a.1 - b.1;
                    let dz = a.2 - b.2;
                    return (dx*dx+dy*dy+dz*dz).sqrt();
                };
                let mut shortest_edge = f32::INFINITY; 
                for i in 0..(vertex_faces.len()/3){
                    let d1 = dst(vertices[vertex_faces[i*3] as usize],vertices[vertex_faces[i*3 + 1] as usize]);
                    let d2 = dst(vertices[vertex_faces[i*3 + 1] as usize],vertices[vertex_faces[i*3 + 2] as usize]);
                    let d3 = dst(vertices[vertex_faces[i*3 + 2] as usize],vertices[vertex_faces[i*3] as usize]);
                    shortest_edge = shortest_edge.min(d1.min(d2.min(d3)));
                }
                shortest_edge
            },
            None=>1.0,
        };
        // Save vertices
        match &self.vertices{
            Some(vertices)=>{
                use crate::vertices::{VertexPrecisionMode,save_tmf_vertices};
                save_tmf_vertices(vertices,VertexPrecisionMode(0.01),&mut curr_segment_data,shortest_edge)?;
                w.write_all(&(SectionHeader::VertexSegment as u16).to_le_bytes())?;
                w.write_all(&(curr_segment_data.len() as u32).to_le_bytes())?;
                w.write_all(&curr_segment_data)?;
                curr_segment_data.clear();
            },
            None=>(),
        }
        // Save vertex faces
        match &self.vertex_faces{
            Some(vertex_faces)=>{
                use crate::vertices::save_faces;
                //If saving vertex faces, vertices must be present, so unwrap can't fail
                let v_count = self.vertices.as_ref().unwrap().len();
                save_faces(vertex_faces,v_count,&mut curr_segment_data);
                w.write_all(&(SectionHeader::VertexFaceSegment as u16).to_le_bytes())?;
                w.write_all(&(curr_segment_data.len() as u32).to_le_bytes())?;
                w.write_all(&curr_segment_data)?;
                curr_segment_data.clear();
            },
            None=>(),
        };
        // Save Normals
        match &self.normals{
            Some(normals)=>{
                use crate::normals::*;
                save_normal_array(normals,&mut curr_segment_data,NormalPrecisionMode::from_deg_dev(0.1))?;
                w.write_all(&(SectionHeader::NormalSegment as u16).to_le_bytes())?;
                w.write_all(&(curr_segment_data.len() as u32).to_le_bytes())?;
                w.write_all(&curr_segment_data)?;
                curr_segment_data.clear();
            },
            None=>(),
        };
        // Save normal faces
        match &self.normal_faces{
            Some(normal_faces)=>{
                use crate::vertices::save_faces;
                //If saving normal faces, normals must be present, so unwrap can't fail
                let n_count = self.normals.as_ref().unwrap().len();
                save_faces(normal_faces,n_count,&mut curr_segment_data);
                w.write_all(&(SectionHeader::NormalFaceSegment as u16).to_le_bytes())?;
                w.write_all(&(curr_segment_data.len() as u32).to_le_bytes())?;
                w.write_all(&curr_segment_data)?;
                curr_segment_data.clear();
            },
            None=>(),
        };
        match &self.uvs{
            Some(uvs)=>{
                uv::save_uvs(uvs,&mut curr_segment_data,0.001)?;
                w.write_all(&(SectionHeader::UvSegment as u16).to_le_bytes())?;
                w.write_all(&(curr_segment_data.len() as u32).to_le_bytes())?;
                w.write_all(&curr_segment_data)?;
                curr_segment_data.clear();
            },
            None=>(),
        }
        // Save uv faces
        match &self.uv_faces{
            Some(uv_faces)=>{
                use crate::vertices::save_faces;
                //If saving uv faces, uvs must be present, so unwrap can't fail
                let uv_count = self.uvs.as_ref().unwrap().len();
                save_faces(uv_faces,uv_count,&mut curr_segment_data);
                w.write_all(&(SectionHeader::UvFaceSegement as u16).to_le_bytes())?;
                w.write_all(&(curr_segment_data.len() as u32).to_le_bytes())?;
                w.write_all(&curr_segment_data)?;
                curr_segment_data.clear();
            },
            None=>(),
        };
        Ok(())
    }
    pub fn read_tmf<R:Read>(reader:&mut R)->Result<Self>{
        let mut magic = [0;3];
        reader.read_exact(&mut magic)?;
        if magic != *b"TMF"{
            return Err(std::io::Error::new(std::io::ErrorKind::Other,"Not a TMF file")); 
        }
        let major = {
            let mut tmp = [0;2];
            reader.read(&mut tmp)?;
            u16::from_le_bytes(tmp)
        };
        let minor = {
            let mut tmp = [0;2];
            reader.read(&mut tmp)?;
            u16::from_le_bytes(tmp)
        };
        println!("major:{major},minor:{minor}");
        todo!();
    }
}
#[cfg(test)]
mod testing{
    use super::*;
    #[test]
    fn read_susan_obj(){
        let mut file = std::fs::File::open("testing/susan.obj").unwrap();
        let tmf_mesh = TMFMesh::read_from_obj(&mut file).unwrap();
        tmf_mesh.verify().unwrap();
    }
    #[test]
    fn rw_susan_obj(){
        let mut file = std::fs::File::open("testing/susan.obj").unwrap();
        let tmf_mesh = TMFMesh::read_from_obj(&mut file).unwrap();
        tmf_mesh.verify().unwrap();
        let mut out = std::fs::File::create("target/susan.obj").unwrap();
        tmf_mesh.write_obj(&mut out).unwrap();
    }
    #[test]
    fn save_susan_tmf(){
        let mut file = std::fs::File::open("testing/susan.obj").unwrap();
        let tmf_mesh = TMFMesh::read_from_obj(&mut file).unwrap();
        tmf_mesh.verify().unwrap();
        let mut out = std::fs::File::create("target/susan.tmf").unwrap();
        tmf_mesh.write_tmf(&mut out).unwrap();
    }
    #[test]
    fn rw_susan_tmf(){
        let mut file = std::fs::File::open("testing/susan.obj").unwrap();
        let tmf_mesh = TMFMesh::read_from_obj(&mut file).unwrap();
        tmf_mesh.verify().unwrap();
        let mut out = Vec::new();
        {
            tmf_mesh.write_tmf(&mut out).unwrap();
        }
        let r_mesh = TMFMesh::read_tmf(&mut (&out as &[u8])).unwrap(); 
    }
    #[test]
    #[should_panic]
    fn rw_cube_obj_not_triangulated(){
        let mut file = std::fs::File::open("testing/cube.obj").unwrap();
        let tmf_mesh = TMFMesh::read_from_obj(&mut file).unwrap();
        tmf_mesh.verify().unwrap();
        let mut out = std::fs::File::create("target/cube.obj").unwrap();
        tmf_mesh.write_obj(&mut out).unwrap();
    }
}
