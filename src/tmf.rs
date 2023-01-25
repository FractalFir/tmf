#[repr(u16)]
#[derive(Debug)]
pub(crate) enum SectionHeader {
    Invalid = 0,
    VertexSegment = 1,
    VertexFaceSegment = 2,
    NormalSegment = 3,
    NormalFaceSegment = 4,
    UvSegment = 5,
    UvFaceSegment = 6,
    MetadataSegment = 7,
    MaterialInfo = 8,
    MaterialFaces = 9,
}
impl SectionHeader {
    pub fn from_u16(input: u16) -> Self {
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
use crate::{FloatType, TMFMesh, TMFPrecisionInfo, Vector3, TMF_MAJOR, TMF_MINOR};
use std::io::{Read, Result, Write};
pub(crate) fn write_mesh<W: Write>(
    mesh: &TMFMesh,
    w: &mut W,
    p_info: &TMFPrecisionInfo,
    name: &str,
) -> Result<()> {
    write_string(w, name);
    w.write_all(&(mesh.get_segment_count() as u16).to_le_bytes())?;
    /// If needed, prune redundant normal data.
    let (normals, normal_faces) = if mesh.get_normals().is_some()
        && mesh.get_normal_faces().is_some()
        && p_info.prune_normals
    {
        use crate::normals::merge_identical_normals;
        let (normals, normal_faces) = merge_identical_normals(
            mesh.get_normals().unwrap(),
            mesh.get_normal_faces().unwrap(),
            p_info.normal_precision,
        );
        (Some(normals), Some(normal_faces))
    } else {
        let normals = match mesh.get_normals() {
            Some(normals) => Some(normals.into()),
            None => None,
        };
        let normal_faces = match mesh.get_normal_faces() {
            Some(normal_faces) => Some(normal_faces.into()),
            None => None,
        };
        (normals, normal_faces)
    };
    let mut curr_segment_data = Vec::with_capacity(0x100);
    //Calculate shortest edge, or if no edges present, 1.0
    let shortest_edge = match &mesh.vertex_faces {
        Some(vertex_faces) => {
            let vertices = match &mesh.vertices {
                Some(vertices) => vertices,
                None => return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Saving a mesh with face normal index array without normal array is an error.",
                )),
            };
            fn dst(a: Vector3, b: Vector3) -> FloatType {
                let dx = a.0 - b.0;
                let dy = a.1 - b.1;
                let dz = a.2 - b.2;
                (dx * dx + dy * dy + dz * dz).sqrt()
            }
            let mut shortest_edge = FloatType::INFINITY;
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
    match &mesh.vertices {
        Some(vertices) => {
            use crate::vertices::save_tmf_vertices;
            save_tmf_vertices(
                vertices,
                p_info.vertex_precision,
                &mut curr_segment_data,
                shortest_edge,
            )?;
            w.write_all(&(SectionHeader::VertexSegment as u16).to_le_bytes())?;
            w.write_all(&(curr_segment_data.len() as u64).to_le_bytes())?;
            w.write_all(&curr_segment_data)?;
            curr_segment_data.clear();
        }
        None => (),
    }
    // Save vertex faces
    match &mesh.vertex_faces {
        Some(vertex_faces) => {
            use crate::vertices::save_faces;
            //If saving vertex faces, vertices must be present, so unwrap can't fail
            let v_count = mesh.vertices.as_ref().unwrap().len();
            save_faces(vertex_faces, v_count, &mut curr_segment_data)?;
            w.write_all(&(SectionHeader::VertexFaceSegment as u16).to_le_bytes())?;
            w.write_all(&(curr_segment_data.len() as u64).to_le_bytes())?;
            w.write_all(&curr_segment_data)?;
            curr_segment_data.clear();
        }
        None => (),
    };
    // Save Normals
    match normals {
        Some(normals) => {
            use crate::normals::*;
            save_normal_array(
                &normals,
                &mut curr_segment_data,
                NormalPrecisionMode::from_deg_dev(0.01),
            )?;
            w.write_all(&(SectionHeader::NormalSegment as u16).to_le_bytes())?;
            w.write_all(&(curr_segment_data.len() as u64).to_le_bytes())?;
            w.write_all(&curr_segment_data)?;
            curr_segment_data.clear();
        }
        None => (),
    };
    // Save normal faces
    match normal_faces {
        Some(normal_faces) => {
            use crate::vertices::save_faces;
            //If saving normal faces, normals must be present, so unwrap can't fail
            let n_count = mesh.normals.as_ref().unwrap().len();
            save_faces(&normal_faces, n_count, &mut curr_segment_data)?;
            w.write_all(&(SectionHeader::NormalFaceSegment as u16).to_le_bytes())?;
            w.write_all(&(curr_segment_data.len() as u64).to_le_bytes())?;
            w.write_all(&curr_segment_data)?;
            curr_segment_data.clear();
        }
        None => (),
    };
    match &mesh.uvs {
        Some(uvs) => {
            crate::uv::save_uvs(uvs, &mut curr_segment_data, 0.001)?;
            w.write_all(&(SectionHeader::UvSegment as u16).to_le_bytes())?;
            w.write_all(&(curr_segment_data.len() as u64).to_le_bytes())?;
            w.write_all(&curr_segment_data)?;
            curr_segment_data.clear();
        }
        None => (),
    }
    // Save uv faces
    match &mesh.uv_faces {
        Some(uv_faces) => {
            use crate::vertices::save_faces;
            //If saving uv faces, uvs must be present, so unwrap can't fail
            let uv_count = mesh.uvs.as_ref().unwrap().len();
            save_faces(uv_faces, uv_count, &mut curr_segment_data)?;
            w.write_all(&(SectionHeader::UvFaceSegment as u16).to_le_bytes())?;
            w.write_all(&(curr_segment_data.len() as u64).to_le_bytes())?;
            w.write_all(&curr_segment_data)?;
            curr_segment_data.clear();
        }
        None => (),
    };
    Ok(())
}
pub(crate) fn write_string<W: Write>(w: &mut W, s: &str) -> Result<()> {
    let bytes = s.as_bytes();
    w.write_all(&(bytes.len() as u16).to_le_bytes())?;
    w.write_all(bytes)
}
pub(crate) fn read_u16<R: Read>(r: &mut R) -> Result<u16> {
    let mut tmp = [0; std::mem::size_of::<u16>()];
    r.read_exact(&mut tmp)?;
    Ok(u16::from_le_bytes(tmp))
}
pub(crate) fn read_string<R: Read>(r: &mut R) -> Result<String> {
    let byte_len = read_u16(r)?;
    let mut bytes = vec![0; byte_len as usize];
    r.read(&mut bytes)?;
    match std::str::from_utf8(&bytes) {
        Ok(string) => Ok(string.to_owned()),
        Err(_) => todo!(),
    }
}
pub(crate) fn write_tmf_header<W: Write>(w: &mut W, mesh_count: u32) -> Result<()> {
    w.write_all(b"TMF")?;
    w.write_all(&TMF_MAJOR.to_le_bytes())?;
    w.write_all(&TMF_MINOR.to_le_bytes())?;
    w.write_all(&mesh_count.to_le_bytes())
}
pub(crate) fn write<W: Write>(
    meshes_names: &[(TMFMesh, &str)],
    w: &mut W,
    p_info: &TMFPrecisionInfo,
) -> Result<()> {
    write_tmf_header(w, meshes_names.len() as u32)?;
    for (mesh, name) in meshes_names {
        write_mesh(mesh, w, p_info, name)?;
    }
    Ok(())
}
pub fn read_mesh<R: Read>(reader: &mut R) -> Result<(TMFMesh, String)> {
    let mut res = TMFMesh::empty();
    let name = read_string(reader)?;
    let seg_count = read_u16(reader)?;
    for _ in 0..seg_count {
        let header = read_u16(reader)?;
        let header = SectionHeader::from_u16(header);
        let data_length = {
            let mut tmp = [0; std::mem::size_of::<u64>()];
            reader.read_exact(&mut tmp)?;
            u64::from_le_bytes(tmp)
        };
        let mut data = vec![0; data_length as usize];
        reader.read_exact(&mut data)?;
        match header {
            SectionHeader::VertexSegment => {
                use crate::vertices::read_tmf_vertices;
                if res
                    .set_vertices(&read_tmf_vertices(&mut (&data as &[u8]))?)
                    .is_some()
                {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Only one vertex array can be present in a model.",
                    ));
                }
            }
            SectionHeader::NormalSegment => {
                use crate::normals::read_normal_array;
                if res
                    .set_normals(&read_normal_array(&mut (&data as &[u8]))?)
                    .is_some()
                {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Only one normal array can be present in a model.",
                    ));
                }
            }
            SectionHeader::UvSegment => {
                use crate::uv::read_uvs;
                if res.set_uvs(&read_uvs(&mut (&data as &[u8]))?).is_some() {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Only one uv array can be present in a model.",
                    ));
                }
            }
            SectionHeader::VertexFaceSegment => {
                use crate::vertices::read_faces;
                if res
                    .set_vertex_faces(&read_faces(&mut (&data as &[u8]))?)
                    .is_some()
                {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Only one vertex index array(face array) can be present in a model.",
                    ));
                }
            }
            SectionHeader::NormalFaceSegment => {
                use crate::vertices::read_faces;
                if res
                    .set_normal_faces(&read_faces(&mut (&data as &[u8]))?)
                    .is_some()
                {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Only one normal index array(face array) can be present in a model.",
                    ));
                }
            }
            SectionHeader::UvFaceSegment => {
                use crate::vertices::read_faces;
                if res
                    .set_uv_faces(&read_faces(&mut (&data as &[u8]))?)
                    .is_some()
                {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Only one uv index array(face array) can be present in a model.",
                    ));
                }
            }
            _ => (), //Unknown header, ignoring
        }
    }
    //todo!();
    Ok((res, name))
}
pub fn read<R: Read>(reader: &mut R) -> Result<Vec<(TMFMesh, String)>> {
    let mut magic = [0; 3];
    reader.read_exact(&mut magic)?;
    if magic != *b"TMF" {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Not a TMF file",
        ));
    }
    // Not used ATM, but can be used for compatiblity in the future.
    let _major = read_u16(reader)?;
    // Not used ATM, but can be used for compatiblity in the future.
    let _minor = read_u16(reader)?;
    let mesh_count = {
        let mut tmp = [0; std::mem::size_of::<u32>()];
        reader.read_exact(&mut tmp)?;
        u32::from_le_bytes(tmp)
    };
    let mut meshes = Vec::with_capacity(mesh_count as usize);
    for _ in 0..mesh_count {
        meshes.push(read_mesh(reader)?);
    }
    Ok(meshes.into())
}
