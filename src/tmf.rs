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
    pub fn is_triangle(&self) -> bool {
        matches!(
            self,
            Self::VertexTriangleSegment
                | Self::NormalTriangleSegment
                | Self::UvTriangleSegment
                | Self::ColorTriangleSegment
                | Self::TangentTriangleSegment
        )
    }
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
use std::io::Write;
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
