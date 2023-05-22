use crate::TMFImportError;
#[repr(u16)]
#[derive(Debug, PartialEq)]
pub(crate) enum SectionType {
    Invalid = 0,
    VertexSegment = 1,
    VertexTriangleSegment = 2,
    NormalSegment = 3,
    NormalTriangleSegment = 4,
    UvSegment = 5,
    UvTriangleSegment = 6,
    ColorSegment = 7,
    ColorTriangleSegment = 8,
    TangentSegment = 9,
    TangentTriangleSegment = 10,
    MaterialInfo = 11,
    MaterialTriangleRanges = 12,
    GroupInfo = 13,
    GroupTriangleRanges = 14,
    CustomIndexSegment = 15,
    CustomFloatSegment = 16,
    CustomUnit2Segment = 17,
    CustomUnit3Segment = 18,
    CustomVector2Segment = 19,
    CustomVector3Segment = 20,
    CustomVector4Segment = 21,
    CustomColorSegment = 23,
}
use crate::{MAX_MESH_COUNT, MAX_SEG_SIZE};
impl SectionType {
    pub fn from_u16(input: u16) -> Self {
        if input > u8::MAX as u16 {
            Self::Invalid
        } else {
            Self::from_u8(input as u8)
        }
    }
    pub fn from_u8(input: u8) -> Self {
        match input {
            1 => Self::VertexSegment,
            2 => Self::VertexTriangleSegment,
            3 => Self::NormalSegment,
            4 => Self::NormalTriangleSegment,
            5 => Self::UvSegment,
            6 => Self::UvTriangleSegment,
            15 => Self::CustomIndexSegment,
            16 => Self::CustomFloatSegment,
            _ => Self::Invalid,
        }
    }
}
#[repr(u8)]
#[derive(PartialEq)]
pub(crate) enum CompressionType {
    None = 0,
    Ommited = 1,
    UnalignedLZZ = 2,
}
impl CompressionType {
    pub fn from_u8(input: u8) -> Result<Self, TMFImportError> {
        match input {
            0 => Ok(Self::None),
            1 => Ok(Self::Ommited),
            2 => Ok(Self::UnalignedLZZ),
            _ => Err(TMFImportError::CompressionTypeUnknown(input)),
        }
    }
}

fn read_segment_header<R: Read>(
    reader: &mut R,
) -> Result<(SectionType, usize, CompressionType), TMFImportError> {
    let seg_type = read_u16(reader)?;
    let seg_type = SectionType::from_u16(seg_type);
    let data_length = {
        let mut tmp = [0; std::mem::size_of::<u64>()];
        reader.read_exact(&mut tmp)?;
        u64::from_le_bytes(tmp)
    };
    let compression_type = {
        let mut tmp = [0; 1];
        reader.read_exact(&mut tmp)?;
        CompressionType::from_u8(tmp[0])?
    };
    Ok((seg_type, data_length as usize, compression_type))
}
fn write_segment_header<W: Write>(
    w: &mut W,
    seg_type: SectionType,
    data_length: usize,
    comperssion_type: CompressionType,
) -> std::io::Result<()> {
    w.write_all(&(seg_type as u16).to_le_bytes())?;
    w.write_all(&(data_length as u64).to_le_bytes())?;
    w.write_all(&[comperssion_type as u8])
}
use crate::{
    FloatType, IndexType, TMFMesh, TMFPrecisionInfo, Vector3, MIN_TMF_MAJOR, MIN_TMF_MINOR,
    TMF_MAJOR, TMF_MINOR,
};
fn calc_shortest_edge(
    vertex_triangles: Option<&[IndexType]>,
    vertices: Option<&[Vector3]>,
) -> std::io::Result<FloatType> {
    let shortest_edge = match vertex_triangles {
        Some(vertex_triangles) => {
            if vertex_triangles.is_empty() {
                //TODO: handle 0 faced mesh as mesh with no faces!
                return Ok(1.0);
            }
            use crate::utilis::distance;
            let vertices = match vertices {
                Some(vertices) => vertices,
                None => return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Saving a mesh with triangle vertex array without normal array is an error.",
                )),
            };
            let mut shortest_edge = FloatType::INFINITY;
            for i in 0..(vertex_triangles.len() / 3) {
                let d1 = distance(
                    vertices[vertex_triangles[i * 3] as usize],
                    vertices[vertex_triangles[i * 3 + 1] as usize],
                );
                let d2 = distance(
                    vertices[vertex_triangles[i * 3 + 1] as usize],
                    vertices[vertex_triangles[i * 3 + 2] as usize],
                );
                let d3 = distance(
                    vertices[vertex_triangles[i * 3 + 2] as usize],
                    vertices[vertex_triangles[i * 3] as usize],
                );
                shortest_edge = shortest_edge.min(d1.min(d2.min(d3)));
            }
            shortest_edge
        }
        // TODO: Calculate distance between closest points for point cloud
        None => 1.0,
    };
    assert!(
        shortest_edge.is_finite(),
        "Shortest edge should be finite but is '{shortest_edge}'!"
    );
    Ok(shortest_edge)
}
fn save_normals<W: Write>(
    normals: &Option<Vec<Vector3>>,
    w: &mut W,
    curr_segment_data: &mut Vec<u8>,
    p_info: &TMFPrecisionInfo,
) -> std::io::Result<()> {
    // Save Normals
    match normals {
        Some(normals) => {
            use crate::normals::*;
            save_normal_array(normals, curr_segment_data, p_info.normal_precision)?;
            write_segment_header(
                w,
                SectionType::NormalSegment,
                curr_segment_data.len(),
                CompressionType::None,
            )?;
            w.write_all(curr_segment_data)?;
            curr_segment_data.clear();
        }
        None => (),
    };
    Ok(())
}
fn save_vertices<W: Write>(
    vertices: Option<&[Vector3]>,
    w: &mut W,
    curr_segment_data: &mut Vec<u8>,
    p_info: &TMFPrecisionInfo,
    shortest_edge: FloatType,
) -> std::io::Result<()> {
    if let Some(vertices) = vertices {
        use crate::vertices::save_tmf_vertices;
        save_tmf_vertices(
            vertices,
            p_info.vertex_precision,
            curr_segment_data,
            shortest_edge,
        )?;
        write_segment_header(
            w,
            SectionType::VertexSegment,
            curr_segment_data.len(),
            CompressionType::None,
        )?;
        w.write_all(curr_segment_data)?;
        curr_segment_data.clear();
    }
    Ok(())
}
fn save_normals_and_normal_triangles<W: Write>(
    w: &mut W,
    normals: Option<Vec<Vector3>>,
    normal_triangles: Option<Vec<IndexType>>,
    curr_segment_data: &mut Vec<u8>,
    p_info: &TMFPrecisionInfo,
) -> std::io::Result<()> {
    use crate::normals::get_predicted_normal_array_size;
    // Calculate size of the normal array
    let normal_arr_size = match normals {
        Some(ref normals) => {
            get_predicted_normal_array_size(p_info.normal_precision, normals.len())
        }
        None => {
            if normal_triangles.is_some() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Normal triangle index array can't be present without the normal array.",
                ));
            }
            return Ok(());
        }
    };
    // Calculate size of index array in different compression variants
    let (index_arr_size, index_norm_size) = match normal_triangles {
        Some(ref normal_triangles) => {
            // Unwrap can't fail, or else would have returned previously.
            let normals = normals.as_ref().unwrap();
            let precision = (normals.len() as f64).log2().ceil() as usize;
            (
                (normal_triangles.len() * precision + 8 - 1) / 8 + 1 + std::mem::size_of::<u64>(),
                get_predicted_normal_array_size(p_info.normal_precision, normal_triangles.len()),
            )
        }
        None => return save_normals(&normals, w, curr_segment_data, p_info),
    };
    let no_compression = normal_arr_size + index_arr_size;
    let omitted = index_norm_size;

    if no_compression > omitted {
        // Unwraps can't fail, or else would have returned previously.
        let normal_triangles = normal_triangles.unwrap();
        let normals = normals.unwrap();

        let mut new_normals = Vec::with_capacity(normal_triangles.len());
        for index in normal_triangles {
            new_normals.push(normals[index as usize]);
        }
        save_normals(&Some(new_normals), w, curr_segment_data, p_info)?;
        write_segment_header(
            w,
            SectionType::NormalTriangleSegment,
            0,
            CompressionType::Ommited,
        )?;
        return Ok(());
    }
    // Save Normals
    save_normals(&normals, w, curr_segment_data, p_info)?;
    // Save normal triangles
    if let Some(normal_triangles) = normal_triangles {
        use crate::vertices::save_triangles;
        //If saving normal triangles, normals must be present, so unwrap can't fail
        let n_count = normals.as_ref().unwrap().len();
        save_triangles(&normal_triangles, n_count, curr_segment_data)?;
        write_segment_header(
            w,
            SectionType::NormalTriangleSegment,
            curr_segment_data.len(),
            CompressionType::None,
        )?;
        w.write_all(curr_segment_data)?;
        curr_segment_data.clear();
    };
    Ok(())
}
use crate::normals::map_prune;
use std::io::{Read, Write};
pub(crate) fn write_mesh<W: Write>(
    mesh: &TMFMesh,
    w: &mut W,
    p_info: &TMFPrecisionInfo,
    name: &str,
) -> std::io::Result<()> {
    write_string(w, name)?;
    w.write_all(&(mesh.get_segment_count() as u16).to_le_bytes())?;
    // If needed, prune redundant normal data.
    let (normals, normal_triangles) = if mesh.get_normals().is_some()
        && mesh.get_normal_triangles().is_some()
        && p_info.prune_normals
    {
        let mut normals: Vec<Vector3> = mesh.get_normals().unwrap().into();
        let mut normal_triangles: Vec<IndexType> = mesh.get_normal_triangles().unwrap().into();
        map_prune(&mut normals, &mut normal_triangles, 0x1_00_00_00, p_info);
        (Some(normals), Some(normal_triangles))
    } else {
        //Option<[]> to Option<Vec<>>
        let normals = mesh.get_normals().map(|normals| normals.into());
        //Option<[]> to Option<Vec<>>
        let normal_triangles = mesh
            .get_normal_triangles()
            .map(|normal_triangles| normal_triangles.into());
        (normals, normal_triangles)
    };
    let mut curr_segment_data = Vec::with_capacity(0x100);
    //Calculate shortest edge, or if no edges present, 1.0
    let shortest_edge = calc_shortest_edge(mesh.get_vertex_triangles(), mesh.get_vertices())?;
    // Save vertices
    save_vertices(
        mesh.get_vertices(),
        w,
        &mut curr_segment_data,
        p_info,
        shortest_edge,
    )?;
    // Save vertex triangles
    match &mesh.vertex_triangles {
        Some(vertex_triangles) => {
            use crate::vertices::save_triangles;
            //If saving vertex triangles, vertices must be present, so unwrap can't fail
            let v_count = mesh.vertices.as_ref().unwrap().len();
            save_triangles(vertex_triangles, v_count, &mut curr_segment_data)?;
            write_segment_header(
                w,
                SectionType::VertexTriangleSegment,
                curr_segment_data.len(),
                CompressionType::None,
            )?;
            w.write_all(&curr_segment_data)?;
            curr_segment_data.clear();
        }
        None => (),
    };
    save_normals_and_normal_triangles(
        w,
        normals,
        normal_triangles,
        &mut curr_segment_data,
        p_info,
    )?;
    match &mesh.uvs {
        Some(uvs) => {
            crate::uv::save_uvs(uvs, &mut curr_segment_data, 0.001)?;
            write_segment_header(
                w,
                SectionType::UvSegment,
                curr_segment_data.len(),
                CompressionType::None,
            )?;
            w.write_all(&curr_segment_data)?;
            curr_segment_data.clear();
        }
        None => (),
    }
    // Save uv triangles
    match &mesh.uv_triangles {
        Some(uv_triangles) => {
            use crate::vertices::save_triangles;
            //If saving uv triangles, uvs must be present, so unwrap can't fail
            let uv_count = mesh.uvs.as_ref().unwrap().len();
            save_triangles(uv_triangles, uv_count, &mut curr_segment_data)?;
            write_segment_header(
                w,
                SectionType::UvTriangleSegment,
                curr_segment_data.len(),
                CompressionType::None,
            )?;
            w.write_all(&curr_segment_data)?;
            curr_segment_data.clear();
        }
        None => (),
    };
    for data in &mesh.custom_data {
        data.write(&mut curr_segment_data)?;
        w.write_all(&curr_segment_data)?;
        curr_segment_data.clear();
    }
    Ok(())
}
pub(crate) fn write_string<W: Write>(w: &mut W, s: &str) -> std::io::Result<()> {
    let bytes = s.as_bytes();
    w.write_all(&(bytes.len() as u16).to_le_bytes())?;
    w.write_all(bytes)
}
pub(crate) fn read_u16<R: Read>(r: &mut R) -> std::io::Result<u16> {
    let mut tmp = [0; std::mem::size_of::<u16>()];
    r.read_exact(&mut tmp)?;
    Ok(u16::from_le_bytes(tmp))
}
pub(crate) fn read_string<R: Read>(r: &mut R) -> std::io::Result<String> {
    let byte_len = read_u16(r)?;
    let mut bytes = vec![0; byte_len as usize];
    r.read_exact(&mut bytes)?;
    match std::str::from_utf8(&bytes) {
        Ok(string) => Ok(string.to_owned()),
        Err(_) => Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Mesh name not valid UTF-8",
        )),
    }
}
pub(crate) fn write_tmf_header<W: Write>(w: &mut W, mesh_count: u32) -> std::io::Result<()> {
    w.write_all(b"TMF")?;
    w.write_all(&TMF_MAJOR.to_le_bytes())?;
    w.write_all(&TMF_MINOR.to_le_bytes())?;
    w.write_all(&MIN_TMF_MAJOR.to_le_bytes())?;
    w.write_all(&MIN_TMF_MINOR.to_le_bytes())?;
    w.write_all(&mesh_count.to_le_bytes())
}
pub(crate) fn write<W: Write, S: std::borrow::Borrow<str>>(
    meshes_names: &[(TMFMesh, S)],
    w: &mut W,
    p_info: &TMFPrecisionInfo,
) -> std::io::Result<()> {
    write_tmf_header(w, meshes_names.len() as u32)?;
    for (mesh, name) in meshes_names {
        write_mesh(mesh, w, p_info, name.borrow())?;
    }
    Ok(())
}
fn read_normal_faces(
    mesh: &mut TMFMesh,
    compression_type: CompressionType,
    data: &[u8],
) -> Result<(), TMFImportError> {
    use crate::vertices::read_triangles;
    // If normal triangle indices are omitted, assume that normal array is already laid out in triangles.
    let normal_triangles = if compression_type == CompressionType::Ommited {
        let normals_len = match mesh.get_normals() {
            Some(normals) => normals.len(),
            None => return Err(TMFImportError::NoDataBeforeOmmitedSegment),
        };
        let mut normal_triangles = Vec::with_capacity(normals_len);
        for index in 0..normals_len {
            normal_triangles.push(index as IndexType);
        }
        normal_triangles.into()
    } else {
        read_triangles(&mut (data as &[u8]))?
    };
    if mesh.set_normal_triangles(normal_triangles).is_some() {
        //TODO:Support multiple arrays inside one mesh!
        return Err(TMFImportError::IO(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Only one normal index array(triangle array) can be present in a model.",
        )));
    }
    Ok(())
}
pub fn read_mesh<R: Read>(reader: &mut R) -> Result<(TMFMesh, String), TMFImportError> {
    let mut res = TMFMesh::empty();
    let name = read_string(reader)?;
    let seg_count = read_u16(reader)?;
    for _ in 0..seg_count {
        let (seg_type, data_length, compression_type) = read_segment_header(reader)?;
        if data_length > MAX_SEG_SIZE {
            return Err(TMFImportError::SegmentTooLong);
        }
        let mut data = vec![0; data_length];
        reader.read_exact(&mut data)?;
        //DEBUG: Temporary segment sizes in bytes
        //println!("{seg_type:?}:{data_length} bytes.");
        match seg_type {
            SectionType::VertexSegment => {
                use crate::vertices::read_tmf_vertices;
                if res
                    .set_vertices(read_tmf_vertices(&mut (&data as &[u8]))?)
                    .is_some()
                {
                    //TODO: Append instead of returning error
                    return Err(TMFImportError::IO(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Only one vertex array can be present in a model.",
                    )));
                }
            }
            SectionType::NormalSegment => {
                use crate::normals::read_normal_array;
                if res
                    .set_normals(read_normal_array(&mut (&data as &[u8]))?)
                    .is_some()
                {
                    //TODO: Append instead of returning error
                    return Err(TMFImportError::IO(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Only one normal array can be present in a model.",
                    )));
                }
            }
            SectionType::UvSegment => {
                use crate::uv::read_uvs;
                if res.set_uvs(read_uvs(&mut (&data as &[u8]))?).is_some() {
                    //TODO: Append instead of returning error
                    return Err(TMFImportError::IO(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Only one uv array can be present in a model.",
                    )));
                }
            }
            SectionType::VertexTriangleSegment => {
                use crate::vertices::read_triangles;
                if res
                    .set_vertex_triangles(read_triangles(&mut (&data as &[u8]))?)
                    .is_some()
                {
                    //TODO: Append instead of returning error
                    return Err(TMFImportError::IO(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Only one vertex index array(triangle array) can be present in a model.",
                    )));
                }
            }
            SectionType::NormalTriangleSegment => {
                read_normal_faces(&mut res, compression_type, &data)?
            }
            SectionType::UvTriangleSegment => {
                use crate::vertices::read_triangles;
                if res
                    .set_uv_triangles(read_triangles(&mut (&data as &[u8]))?)
                    .is_some()
                {
                    return Err(TMFImportError::IO(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Only one uv index array(triangle array) can be present in a model.",
                    )));
                }
            }
            SectionType::CustomIndexSegment
            | SectionType::CustomFloatSegment
            | SectionType::CustomUnit2Segment
            | SectionType::CustomUnit3Segment
            | SectionType::CustomVector2Segment
            | SectionType::CustomVector3Segment
            | SectionType::CustomVector4Segment
            | SectionType::CustomColorSegment => {
                let cd = crate::custom_data::CustomDataSegment::read(&*data, seg_type)?;
                res.add_custom_data_seg(cd);
            }
            _ => (), //Unknown header, ignoring
        }
    }
    //todo!();
    Ok((res, name))
}
pub fn read<R: Read>(reader: &mut R) -> Result<Vec<(TMFMesh, String)>, TMFImportError> {
    let mut magic = [0; 3];
    reader.read_exact(&mut magic)?;
    if magic != *b"TMF" {
        return Err(TMFImportError::NotTMFFile);
    }
    // Not used ATM, but can be used for compatiblity in the future.
    let _major = read_u16(reader)?;
    // Not used ATM, but can be used for compatiblity in the future.
    let _minor = read_u16(reader)?;
    // Minimum version of reader required to read
    let min_major = read_u16(reader)?;
    let min_minor = read_u16(reader)?;
    if min_major > TMF_MAJOR || (min_major == TMF_MAJOR && min_minor > TMF_MINOR) {
        return Err(TMFImportError::NewerVersionRequired);
    }
    let mesh_count = {
        let mut tmp = [0; std::mem::size_of::<u32>()];
        reader.read_exact(&mut tmp)?;
        u32::from_le_bytes(tmp)
    };
    if mesh_count > MAX_MESH_COUNT as u32 {
        return Err(TMFImportError::TooManyMeshes);
    }
    let mut meshes = Vec::with_capacity(mesh_count as usize);
    for _ in 0..mesh_count {
        meshes.push(read_mesh(reader)?);
    }
    Ok(meshes)
}
