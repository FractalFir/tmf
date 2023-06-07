use crate::tmf::DecodedSegment;
use crate::read_extension::ReadExt;
use smallvec::{SmallVec,smallvec};
use crate::tmf::{SectionType,EncodedSegment,CompressionType};
use crate::TMFImportError;
use crate::{
    FloatType, IndexType, TMFExportError, TMFMesh, TMFPrecisionInfo, Vector3, MIN_TMF_MAJOR,
    MIN_TMF_MINOR, TMF_MAJOR, TMF_MINOR, CustomDataSegment,MAX_SEG_SIZE,
};
use crate::unaligned_rw::{UnalignedRWMode,UnalignedReader};
pub(crate) struct EncodeInfo {
    shortest_edge: FloatType,
}
impl Default for EncodeInfo {
    fn default() -> Self {
        Self { shortest_edge: 0.1 }
    }
}
impl EncodeInfo {
    pub(crate) fn shortest_edge(&self) -> FloatType {
        self.shortest_edge
    }
}
fn calc_shortest_edge(
    vertex_triangles: Option<&[IndexType]>,
    vertices: Option<&[Vector3]>,
) -> FloatType {
    let shortest_edge = match vertex_triangles {
        Some(vertex_triangles) => {
            if vertex_triangles.is_empty() {
                //TODO: handle 0 faced mesh as mesh with no faces!
                return 0.1;
            }
            use crate::utilis::distance;
            let vertices = match vertices {
                Some(vertices) => vertices,
                None => return 0.1,
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
        None => 0.1,
    };
    assert!(
        shortest_edge.is_finite(),
        "Shortest edge should be finite but is '{shortest_edge}'!"
    );
    shortest_edge
}

pub(crate) fn write_mesh_name<W: std::io::Write>(w: &mut W, s: &str) -> std::io::Result<()> {
    let bytes = s.as_bytes();
    w.write_all(&(bytes.len() as u16).to_le_bytes())?;
    w.write_all(bytes)
}
fn merge_segments(segments:&[DecodedSegment])->Box<[DecodedSegment]>{
    let mut segments:Vec<_> = segments.into();
    let mut new_segments = Vec::with_capacity(segments.len());
    while let Some(mut segment) = segments.pop(){
        if segment.is_something(){
            for mut other in &mut segments{
                segment.merge(&mut other);      
            }
            new_segments.push(segment);
        }
    }
    new_segments.into()
}
async fn write_mesh<W: std::io::Write>(
    mesh: &TMFMesh,
    name: &str,
    target: &mut W,
    p_info: &TMFPrecisionInfo,
) -> Result<(), TMFExportError> {
    write_mesh_name(target, name)?;
    let ei = EncodeInfo {
        shortest_edge: calc_shortest_edge(mesh.get_vertex_triangles(), mesh.get_vertices()),
    };
    let tmf_segs = merge_segments(&MeshSegIter::tmf_segs(mesh).collect::<Box<[_]>>());
    let mut new_segs = Vec::with_capacity(32);
    for seg in tmf_segs.into_iter(){
        let c_segs = seg.clone().optimize().await;
        for c_seg in c_segs {
            new_segs.push(c_seg);
        }
    }
    let tmf_segs = new_segs;
    let mut encoded = Vec::with_capacity(tmf_segs.len());
    for seg in tmf_segs {
        encoded.push(seg.encode(p_info, &ei));
    }
    let encoded = futures::future::join_all(encoded).await;
    target.write_all(&(encoded.len() as u16).to_le_bytes())?;
    for seg in encoded {
        seg?.write(target)?;
    }
    Ok(())
}
pub(crate) async fn write_tmf<W: std::io::Write, S: std::borrow::Borrow<str>>(
    meshes_names: &[(TMFMesh, S)],
    target: &mut W,
    p_info: &TMFPrecisionInfo,
) -> Result<(), TMFExportError> {
    let mesh_count = meshes_names.len();
    write_tmf_header(target, mesh_count as u32)?;
    for (mesh, name) in meshes_names {
        write_mesh(mesh, name.borrow(), target, p_info).await?;
    }
    Ok(())
}
pub(crate) fn write_tmf_header<W: std::io::Write>(
    w: &mut W,
    mesh_count: u32,
) -> Result<(), TMFExportError> {
    w.write_all(b"TMF")?;
    w.write_all(&TMF_MAJOR.to_le_bytes())?;
    w.write_all(&(TMF_MINOR).to_le_bytes())?;
    w.write_all(&MIN_TMF_MAJOR.to_le_bytes())?;
    w.write_all(&(MIN_TMF_MINOR).to_le_bytes())?;
    Ok(w.write_all(&mesh_count.to_le_bytes())?)
}
#[cfg(test)]
fn init_test_env() {
    std::fs::create_dir_all("target/test_res").unwrap();
}
struct MeshSegIter<'a> {
    mesh: &'a TMFMesh,
    item: usize,
}
impl<'a> MeshSegIter<'a> {
    fn tmf_segs(mesh: &'a TMFMesh) -> Self {
        Self { mesh, item: 0 }
    }
}
impl<'a> std::iter::Iterator for MeshSegIter<'a> {
    type Item = DecodedSegment;
    fn next(&mut self) -> Option<Self::Item> {
        self.item += 1;
        match self.item {
            0 => panic!("Impossible condition reached."),
            1 => match self.mesh.get_vertices() {
                Some(vertices) => Some(DecodedSegment::AppendVertex(vertices.into())),
                None => self.next(),
            },
            2 => match self.mesh.get_normals() {
                Some(normals) => Some(DecodedSegment::AppendNormal(normals.into())),
                None => self.next(),
            },
            3 => match self.mesh.get_uvs() {
                Some(uvs) => Some(DecodedSegment::AppendUV(uvs.into())),
                None => self.next(),
            },
            4 => match self.mesh.get_vertex_triangles() {
                Some(tris) => Some(DecodedSegment::AppendTriangleVertex(tris.into())),
                None => self.next(),
            },
            5 => match self.mesh.get_normal_triangles() {
                Some(tris) => Some(DecodedSegment::AppendTriangleNormal(tris.into())),
                None => self.next(),
            },
            6 => match self.mesh.get_uv_triangles() {
                Some(tris) => Some(DecodedSegment::AppendTriangleUV(tris.into())),
                None => self.next(),
            },
            7 => match self.mesh.get_tangents() {
                Some(tans) => Some(DecodedSegment::AppendTangent(tans.into())),
                None => self.next(),
            },
            8 => match self.mesh.get_tangent_triangles() {
                Some(tans) => Some(DecodedSegment::AppendTriangleTangent(tans.into())),
                None => self.next(),
            },
            9..=usize::MAX => {
                let index = self.item - 9;
                let seg = self.mesh.custom_data.get(index)?;
                Some(DecodedSegment::AppendCustom(seg.clone()))
            }
            //Should never happen.
            _ => todo!(),
        }
    }
}
#[test]
#[cfg(feature = "obj_import")]
fn rw_susan_tmf() {
    init_test_env();
    let mut file = std::fs::File::open("testing/susan.obj").unwrap();
    let (tmf_mesh, name) = TMFMesh::read_from_obj_one(&mut file).unwrap();
    tmf_mesh.verify().unwrap();
    assert!(name == "Suzanne", "Name should be Suzanne but is {name}");
    let prec = TMFPrecisionInfo::default();
    let mut out = Vec::new();
    {
        futures::executor::block_on(write_tmf(&[(tmf_mesh, name)], &mut out, &prec)).unwrap();
    }
    let (r_mesh, name) = TMFMesh::read_tmf_one(&mut (&out as &[u8])).unwrap();
    assert!(name == "Suzanne", "Name should be Suzanne but is {name}");
    r_mesh.verify().unwrap();
}
const TMF_SEG_SIZE: usize =
    std::mem::size_of::<u8>() + std::mem::size_of::<u32>() + std::mem::size_of::<u8>();
fn calc_spilt_score(len: usize, delta_span: (IndexType, IndexType)) -> isize {
    //return isize::MIN;
    use crate::vertices::calc_prec;
    let gain_bits =
        calc_prec(delta_span.0 as usize) as usize - calc_prec(delta_span.1 as usize) as usize;
    let gain = (gain_bits * len) as isize;
    let loss =
        ((TMF_SEG_SIZE + std::mem::size_of::<u8>() + std::mem::size_of::<u32>()) * 8) as isize;
    gain - loss
}
pub(crate) fn opt_tris(triangles: &[IndexType]) -> SmallVec<[&[IndexType]; 4]> {
    let mut best_score = isize::MIN;
    let mut best_index = usize::MIN;
    let mut max_index = IndexType::MIN;
    let total_max = triangles.iter().max().unwrap_or(&1);
    for (index, tri) in triangles.iter().enumerate() {
        max_index = max_index.max(*tri);
        let score = calc_spilt_score(index, (*total_max, max_index));
        if score > best_score {
            best_index = index;
            best_score = score;
        }
    }
    if best_score > 0 {
        let mut res = SmallVec::new();
        let (s1, s2) = triangles.split_at(best_index);
        let r_1 = opt_tris(s1);
        for seg in r_1 {
            res.push(seg);
        }
        let r_2 = opt_tris(s2);
        for seg in r_2 {
            res.push(seg);
        }
        return res;
    }
    let mut best_score = isize::MIN;
    let mut best_index = usize::MIN;
    let mut min_index = IndexType::MAX;
    let total_min = triangles.iter().min().unwrap_or(&1);
    for (index, tri) in triangles.iter().enumerate() {
        min_index = min_index.min(*tri);
        let score = calc_spilt_score(index, (min_index, *total_min));
        if score > best_score {
            best_index = index;
            best_score = score;
        }
    }
    if best_score > 0 {
        let mut res = SmallVec::new();
        let (s1, s2) = triangles.split_at(best_index);
        let r_1 = opt_tris(s1);
        for seg in r_1 {
            res.push(seg);
        }
        let r_2 = opt_tris(s2);
        for seg in r_2 {
            res.push(seg);
        }
        res
    } else {
        smallvec![triangles]
    }
}
fn range_to_vertex_bit_count(span: std::ops::Range<Vector3>, shortest_edge: FloatType) -> u8 {
    let dx = span.end.0 - span.start.0;
    let dy = span.end.1 - span.start.1;
    let _dz = span.end.2 - span.start.2;
    let inc_x = shortest_edge / dx;
    let inc_y = shortest_edge / dy;
    let inc_z = shortest_edge / dy;
    let prec_x = (1.0 / inc_x).log2().ceil() as u8;
    let prec_y = (1.0 / inc_y).log2().ceil() as u8;
    let prec_z = (1.0 / inc_z).log2().ceil() as u8;
    prec_x + prec_y + prec_z
}
fn expand_vertex_span(span: std::ops::Range<Vector3>, point: Vector3) -> std::ops::Range<Vector3> {
    let min = span.start;
    let max = span.end;
    let min = (min.0.min(point.0), min.1.min(point.1), min.2.min(point.2));
    let max = (max.0.max(point.0), max.1.max(point.1), max.2.max(point.2));
    min..max
}
fn find_best_vertex_spilt(vertices: &[Vector3], shortest_edge: FloatType) -> Option<usize> {
    let mut total_span = (0.0, 0.0, 0.0)..(0.0, 0.0, 0.0);
    vertices.iter().for_each(|point| {
        total_span = expand_vertex_span(total_span.clone(), *point);
    });
    let total_span = total_span;
    let total_per_vertex_bit_count = range_to_vertex_bit_count(total_span, shortest_edge);
    let mut best_split_score = isize::MIN;
    let mut best_split_index = usize::MIN;
    let mut min_span = (0.0, 0.0, 0.0)..(0.0, 0.0, 0.0);
    let mut per_vertex_bit_count = 0;
    for (index, vertex) in vertices.iter().enumerate() {
        if !min_span.contains(vertex) || true {
            min_span = expand_vertex_span(min_span, *vertex);
            per_vertex_bit_count = range_to_vertex_bit_count(min_span.clone(), shortest_edge);
            //println!("vertex:{vertex:?}\t\tmin_span:{min_span:?}");
        } else {
            let gain_per_vert = total_per_vertex_bit_count - per_vertex_bit_count;
            let gain = index * (gain_per_vert as usize);
            let loss = TMF_SEG_SIZE
                + std::mem::size_of::<u64>()
                + 6 * std::mem::size_of::<f64>()
                + 3 * std::mem::size_of::<u8>();
            let score = gain as isize - (loss as isize);
            //println!("{score}");
            if score > best_split_score {
                best_split_index = index;
                best_split_score = score;
            }
        }
    }
    assert!(best_split_index < vertices.len() - 1);
    if best_split_score > 0 {
        Some(best_split_index)
    } else {
        None
    }
}
pub(crate) fn opt_vertices(vertices: &[Vector3]) -> SmallVec<[&[Vector3]; 4]> {
    let _len = vertices.len();
    let split_pos = find_best_vertex_spilt(vertices, 0.01);
    if let Some(split_pos) = split_pos {
        let (i0, i1) = vertices.split_at(split_pos);
        let mut res = SmallVec::new();
        let r_0 = opt_vertices(i0);
        for seg in r_0 {
            res.push(seg);
        }
        let r_1 = opt_vertices(i1);
        for seg in r_1 {
            res.push(seg);
        }
        res
    } else {
        smallvec![vertices]
    }
}
