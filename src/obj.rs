use crate::{FloatType, IndexType, TMFMesh};
use std::io::{BufReader, BufWriter, Read, Result, Write};
fn parse_float_type(float: &str) -> Result<FloatType> {
    match float.parse::<FloatType>() {
        Ok(float) => Ok(float),
        Err(err) => Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            err.to_string(),
        )),
    }
}
fn parse_index(uint: &str) -> Result<IndexType> {
    match uint.parse::<IndexType>() {
        Ok(uint) => Ok(uint),
        Err(err) => Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            err.to_string(),
        )),
    }
}
fn match_split(split: Option<&str>) -> Result<&str> {
    match split {
        Some(beg) => Ok(beg),
        None => Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Invalid .obj line",
        )),
    }
}
use crate::Vector3;
use std::str::Split;
use smallvec::SmallVec;
fn load_indices(split: &mut Split<&[char; 2]>)->Result<(IndexType,IndexType,IndexType)>{
    Ok((parse_index(match_split(split.next())?)? - 1,
    parse_index(match_split(split.next())?)? - 1,
    parse_index(match_split(split.next())?)? - 1))
}
pub fn load_face(split: &mut Split<&[char; 2]>,vertex_faces:&mut Vec<IndexType>,normal_faces:&mut Vec<IndexType>,uv_faces:&mut Vec<IndexType>) -> Result<()>{
    let mut faces:SmallVec<[(IndexType,IndexType,IndexType);6]> = SmallVec::new();
    while let Ok(indices) = load_indices(split){
        faces.push(indices);
    }
    assert!(faces.len() == 3,"Object loader currently supports only loading triangulated faces, but encountered a {} sided polygon!",faces.len());
    //TODO: do triangulation
    for face in faces{
        vertex_faces.push(face.0);
        normal_faces.push(face.2);
        uv_faces.push(face.1);
    }
    Ok(())
}
pub fn load_vec3(split: &mut Split<&[char; 2]>) -> Result<Vector3> {
    let (x, y, z) = (
        match_split(split.next())?,
        match_split(split.next())?,
        match_split(split.next())?,
    );
    Ok((
        parse_float_type(x)?,
        parse_float_type(y)?,
        parse_float_type(z)?,
    ))
}
pub fn read_from_obj<R: Read>(reader: &mut R) -> Result<TMFMesh> {
    use std::io::BufRead;
    let reader = BufReader::new(reader);
    let mut vertices = Vec::with_capacity(0x100);
    let mut normals = Vec::with_capacity(0x100);
    let mut uvs = Vec::with_capacity(0x100);

    let mut vertex_faces = Vec::with_capacity(0x100);
    let mut normal_faces = Vec::with_capacity(0x100);
    let mut uv_faces = Vec::with_capacity(0x100);
    let mut res = TMFMesh::empty();
    for line in reader.lines() {
        let line = line?;
        let mut split = line.split(&[' ', '/']);
        let beg = match_split(split.next())?;
        match beg {
            "v" => {
                vertices.push(load_vec3(&mut split)?);
            }
            "vn" => {
                normals.push(load_vec3(&mut split)?);
            }
            "vt" => {
                let (x, y) = (match_split(split.next())?, match_split(split.next())?);
                let uv = (parse_float_type(x)?, parse_float_type(y)?);
                uvs.push(uv);
            }
            "f" => {
                 load_face(&mut split,&mut vertex_faces,&mut normal_faces,&mut uv_faces);
            }
            "#" => continue,
            "mtllib" => continue, //TODO:use material info
            "o" => continue,      //TODO:use object info
            "s" => continue,      //TODO:use smoothness  info
            _ => todo!("{}", line),
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
pub fn write_obj<W: Write>(tmf_mesh: &TMFMesh, w: &mut W) -> Result<()> {
    let mut w = BufWriter::new(w);
    match &tmf_mesh.get_vertices() {
        None => (),
        Some(vertices) => {
            for vertex in vertices.iter() {
                writeln!(w, "v {} {} {}", vertex.0, vertex.1, vertex.2)?;
            }
        }
    }
    match &tmf_mesh.get_normals() {
        None => (),
        Some(normals) => {
            for normal in normals.iter() {
                writeln!(w, "vn {} {} {}", normal.0, normal.1, normal.2)?;
            }
        }
    }
    match &tmf_mesh.get_uvs() {
        None => (),
        Some(uvs) => {
            for uv in uvs.iter() {
                writeln!(w, "vt {} {}", uv.0, uv.1)?;
            }
        }
    }
    match &tmf_mesh.get_vertex_faces() {
        None => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Vertex faces array must be present when saving to .obj file",
            ))
        }
        Some(vertex_faces) => {
            let normal_faces = match &tmf_mesh.normal_faces {
                Some(normal_faces) => normal_faces,
                None => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Normal face arrays must be present when saving to .obj file",
                    ))
                }
            };
            let uv_faces = match &tmf_mesh.uv_faces {
                Some(uv_faces) => uv_faces,
                None => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "UV face arrays must be present when saving to .obj file",
                    ))
                }
            };
            if vertex_faces.len() != normal_faces.len() || vertex_faces.len() != uv_faces.len() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!(
                        "Face Array size mismatch v:{} n:{} u:{}",
                        vertex_faces.len(),
                        normal_faces.len(),
                        uv_faces.len()
                    ),
                ));
            }
            for i in 0..vertex_faces.len() {
                if i % 3 == 0 {
                    write!(w, "\nf ")?
                };
                write!(
                    w,
                    "{}/{}/{} ",
                    vertex_faces[i] + 1,
                    uv_faces[i] + 1,
                    normal_faces[i] + 1
                )?;
            }
        }
    }
    w.flush()?;
    Ok(())
}
