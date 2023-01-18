use std::io::{Read,Write,Result,BufReader,BufWriter};
use crate::TMFMesh; 
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
fn match_split(split:Option<&str>)->Result<&str>{
    match split{
        Some(beg)=>Ok(beg),
       	None=>Err(std::io::Error::new(std::io::ErrorKind::Other,"Invalid .obj line")),
    }
}
pub fn read_from_obj<R:Read>(reader:&mut R)->Result<TMFMesh>{
	use std::io::BufRead;
    let reader = BufReader::new(reader);
    let mut vertices      = Vec::with_capacity(0x100);
    let mut normals       = Vec::with_capacity(0x100);
    let mut uvs           = Vec::with_capacity(0x100);
    let mut vertex_faces  = Vec::with_capacity(0x100);
    let mut normal_faces  = Vec::with_capacity(0x100);
    let mut uv_faces      = Vec::with_capacity(0x100);
	let mut res = TMFMesh::empty();
    for line in reader.lines(){
        let line = line?; 
        let mut split = line.split(&[' ','/']);
        let beg = match_split(split.next())?;
        match beg{
                "v"=>{
                    let (x,y,z) = (match_split(split.next())?,match_split(split.next())?,match_split(split.next())?);
                    let vertex = (parse_f32(x)?,parse_f32(y)?,parse_f32(z)?);
                    vertices.push(vertex);
                },
                "vn"=>{
                    let (x,y,z) = (match_split(split.next())?,match_split(split.next())?,match_split(split.next())?);
                    let normal = (parse_f32(x)?,parse_f32(y)?,parse_f32(z)?);
                    normals.push(normal);
                },
                "vt"=>{
                    let (x,y) = (match_split(split.next())?,match_split(split.next())?);
                    let uv = (parse_f32(x)?,parse_f32(y)?);
                    uvs.push(uv);
                },
                "f"=>{
                     let (v0,vt0,vn0,v1,vt1,vn1,v2,vt2,vn2) = (
                     match_split(split.next())?,match_split(split.next())?,match_split(split.next())?,
                     match_split(split.next())?,match_split(split.next())?,match_split(split.next())?,
                     match_split(split.next())?,match_split(split.next())?,match_split(split.next())?
                     );
                     if split.next().is_some(){
                        return Err(std::io::Error::new(std::io::ErrorKind::Other,"OBJ reader supports only triangulated meshes ATM."));
                     }
                     vertex_faces.push(parse_u32(v0)?  - 1);
                     vertex_faces.push(parse_u32(v1)?  - 1);
                     vertex_faces.push(parse_u32(v2)?  - 1);
                     normal_faces.push(parse_u32(vn0)? - 1);
                     normal_faces.push(parse_u32(vn1)? - 1);
                     normal_faces.push(parse_u32(vn2)? - 1);
                     uv_faces.push(parse_u32(vt0)?     - 1);
                     uv_faces.push(parse_u32(vt1)?     - 1);
                     uv_faces.push(parse_u32(vt2)?     - 1);
                }
                "#"=>continue,
                "mtllib"=>continue,//TODO:use material info
                "o"=>continue,//TODO:use object info
                "s"=>continue,//TODO:use smoothness  info
                _=>todo!("{}",line),
            }
        }
        res.set_vertices(&vertices);
		res.set_normals(&normals);
		res.set_uvs(&uvs);
		res.set_vertex_faces(&vertex_faces);
		res.set_normal_faces(&normal_faces);
		res.set_uv_faces(&uv_faces);
		Ok(res)
	} 
/// Writes this TMF  mesh to a .obj file.
    pub fn write_obj<W:Write>(tmf_mesh:&TMFMesh,w:&mut W)->Result<()>{
		let mut w = BufWriter::new(w);
        match &tmf_mesh.get_vertices(){
            None=>(),
            Some(vertices)=>{
                for vertex in vertices.iter(){
                    writeln!(w,"v {} {} {}",vertex.0,vertex.1,vertex.2)?;
                }
            }
        }
        match &tmf_mesh.get_normals(){
            None=>(),
            Some(normals)=>{
                for normal in normals.iter(){
                    writeln!(w,"vn {} {} {}",normal.0,normal.1,normal.2)?;
                }
            }
        }
        match &tmf_mesh.get_uvs(){
            None=>(),
            Some(uvs)=>{
                for uv in uvs.iter(){
                    writeln!(w,"vt {} {}",uv.0,uv.1)?;
                }
            }
        }
        match &tmf_mesh.get_vertex_faces(){
            None=>return Err(std::io::Error::new(std::io::ErrorKind::Other,"Vertex faces array must be present when saving to .obj file")),
            Some(vertex_faces)=>{
                let normal_faces = match &tmf_mesh.normal_faces{
                    Some(normal_faces)=>normal_faces,
                    None=>return Err(std::io::Error::new(std::io::ErrorKind::Other,"Normal face arrays must be present when saving to .obj file")),
                };
                let uv_faces = match &tmf_mesh.uv_faces{
                    Some(uv_faces)=>uv_faces,
                    None=>return Err(std::io::Error::new(std::io::ErrorKind::Other,"UV face arrays must be present when saving to .obj file")),
                };
                if vertex_faces.len() != normal_faces.len() || vertex_faces.len() != uv_faces.len(){
                    return Err(std::io::Error::new(std::io::ErrorKind::Other,format!("Face Array size mismatch v:{} n:{} u:{}",vertex_faces.len(),normal_faces.len(),uv_faces.len())));
                }
                for i in 0..vertex_faces.len(){
                    if i%3 == 0{writeln!(w)?};
                    write!(w,"{}/{}/{} ",vertex_faces[i] + 1,uv_faces[i] + 1,normal_faces[i] + 1)?;
                    if i%3 == 2{writeln!(w)?};
                }
            }
        }
		w.flush()?;
        Ok(())
    }
