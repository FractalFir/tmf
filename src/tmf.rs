use crate::read_extension::ReadExt;
use crate::tmf_exporter::EncodeInfo;
use crate::tmf_importer::TMFImportContext;
use crate::unaligned_rw::{UnalignedRWMode, UnalignedReader};
use crate::{
    CustomDataSegment, FloatType, IndexType, TMFExportError, TMFImportError, TMFMesh,
    TMFPrecisionInfo, Tangent, Vector2, Vector3, MAX_SEG_SIZE,
};
use smallvec::{smallvec, SmallVec};
#[repr(u16)]
#[derive(Debug, PartialEq, Clone, Copy)]
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
    CustomIntigerSegment = 24,
}
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
            9 => Self::TangentSegment,
            10 => Self::TangentTriangleSegment,
            15 => Self::CustomIndexSegment,
            16 => Self::CustomFloatSegment,
            24 => Self::CustomIntigerSegment,
            _ => Self::Invalid,
        }
    }
}
#[repr(u8)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub(crate) enum CompressionType {
    None = 0,
    Ommited = 1,
    UnalignedLZZ = 2,
    /// Represents data that is laid out sequentially and changes by exactly one with each element.
    Sequence = 255,
}
impl CompressionType {
    pub fn from_u8(input: u8) -> Result<Self, TMFImportError> {
        match input {
            0 => Ok(Self::None),
            1 => Ok(Self::Ommited),
            2 => Ok(Self::UnalignedLZZ),
            255 => Ok(Self::Sequence),
            _ => Err(TMFImportError::CompressionTypeUnknown(input)),
        }
    }
}
#[derive(Debug)]
pub(crate) enum DecodedSegment {
    Nothing,
    AppendVertex(Box<[Vector3]>),
    AppendNormal(Box<[Vector3]>),
    AppendUV(Box<[Vector2]>),
    AppendTangent(Box<[Tangent]>),
    AppendTriangleVertex(Box<[IndexType]>),
    AppendTriangleNormal(Box<[IndexType]>),
    AppendTriangleUV(Box<[IndexType]>),
    AppendTriangleTangent(Box<[IndexType]>),
    AppendCustom(CustomDataSegment),
}
pub(crate) struct EncodedSegment {
    seg_type: SectionType,
    compresion_type: CompressionType,
    data: Box<[u8]>,
}
impl EncodedSegment {
    pub(crate) fn seg_length(&self) -> usize {
        self.data.len()
    }
    pub(crate) fn seg_type(&self) -> SectionType {
        self.seg_type
    }
    pub(crate) fn write<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        let st: u8 = self.seg_type as u16 as u8;
        w.write_all(&[st])?;
        w.write_all(&(self.data.len() as u32).to_le_bytes())?;
        w.write_all(&[self.compresion_type as u8])?;
        w.write_all(&self.data)
    }
    pub(crate) fn read<R: std::io::Read>(
        ctx: &TMFImportContext,
        src: &mut R,
    ) -> Result<Self, TMFImportError> {
        let seg_type = ctx.segment_type_width().read(src)?;
        let data_length = ctx.segment_length_width().read(src)?;
        let compresion_type = CompressionType::from_u8(src.read_u8()?)?;
        let mut data = vec![0; data_length];
        src.read_exact(&mut data)?;
        Ok(Self {
            data: data.into(),
            compresion_type,
            seg_type,
        })
    }
}
async fn decode_vertex_seg(seg: EncodedSegment) -> Result<DecodedSegment, TMFImportError> {
    if SectionType::VertexSegment == seg.seg_type {
        let mut data: &[u8] = &seg.data[..];
        Ok(DecodedSegment::AppendVertex(
            crate::vertices::read_tmf_vertices(&mut data)?,
        ))
    } else {
        panic!("Unreachable condition reached!");
    }
}
async fn decode_uv_seg(seg: EncodedSegment) -> Result<DecodedSegment, TMFImportError> {
    if SectionType::UvSegment == seg.seg_type {
        let mut data: &[u8] = &seg.data[..];
        Ok(DecodedSegment::AppendUV(crate::uv::read_uvs(&mut data)?))
    } else {
        panic!("Unreachable condition reached!");
    }
}
async fn decode_normal_seg(seg: EncodedSegment) -> Result<DecodedSegment, TMFImportError> {
    if SectionType::NormalSegment == seg.seg_type {
        let mut data: &[u8] = &seg.data[..];
        Ok(DecodedSegment::AppendNormal(
            crate::normals::read_normal_array(&mut data)?,
        ))
    } else {
        panic!("Unreachable condition reached!");
    }
}
async fn decode_custom_seg(
    seg: EncodedSegment,
    ctx: &crate::tmf_importer::TMFImportContext,
) -> Result<DecodedSegment, TMFImportError> {
    if matches!(
        seg.seg_type,
        SectionType::CustomIndexSegment | SectionType::CustomFloatSegment
    ) {
        let mut data: &[u8] = &seg.data[..];
        Ok(DecodedSegment::AppendCustom(CustomDataSegment::read(
            &mut data,
            seg.seg_type,
            ctx,
        )?))
    } else {
        panic!("Unreachable condition reached!");
    }
}
fn read_default_triangles<R: std::io::Read>(
    mut src: R,
    data: &mut Vec<IndexType>,
    ctx: &crate::tmf_importer::TMFImportContext,
) -> Result<(), TMFImportError> {
    let precision_bits = src.read_u8()?;
    let length = src.read_u64()?;
    let min = ctx.read_traingle_min(&mut src)?;
    if length > MAX_SEG_SIZE as u64 {
        return Err(TMFImportError::SegmentTooLong);
    }
    data.reserve(length as usize);
    let buf = data.spare_capacity_mut();
    let precision = UnalignedRWMode::precision_bits(precision_bits);
    let mut reader = UnalignedReader::new(src);
    if precision_bits == 0 {
        use std::mem::MaybeUninit;
        buf.fill(MaybeUninit::new(0));
    } else {
        for index in 0..(length as usize) / 2 {
            let (i1, i2) = reader.read2_unaligned(precision)?;
            buf[index * 2].write((i1 + min) as IndexType);
            buf[index * 2 + 1].write((i2 + min) as IndexType);
        }
        if length % 2 != 0 {
            let i = reader.read_unaligned(precision)?;
            buf[(length - 1) as usize].write((i + min) as IndexType);
        }
    }
    unsafe { data.set_len(length as usize) }
    Ok(())
}
fn read_triangle_sequence<R: std::io::Read>(
    _src: R,
    _data: &mut Vec<IndexType>,
) -> Result<(), TMFImportError> {
    todo!();
}
async fn decode_triangle_seg(
    seg: EncodedSegment,
    ctx: &crate::tmf_importer::TMFImportContext,
) -> Result<DecodedSegment, TMFImportError> {
    if seg.seg_type.is_triangle() {
        let data: &[u8] = &seg.data[..];
        let mut indices = Vec::new();
        match seg.compresion_type {
            CompressionType::None => read_default_triangles(data, &mut indices, ctx)?,
            CompressionType::Sequence => read_triangle_sequence(data, &mut indices)?,
            CompressionType::Ommited => panic!("New decoder does not support ommited segment!"),
            CompressionType::UnalignedLZZ => panic!("Unaligned lzz not supported yet!"),
        };
        Ok(match seg.seg_type {
            SectionType::VertexTriangleSegment => {
                DecodedSegment::AppendTriangleVertex(indices.into())
            }
            SectionType::NormalTriangleSegment => {
                DecodedSegment::AppendTriangleNormal(indices.into())
            }
            SectionType::UvTriangleSegment => DecodedSegment::AppendTriangleUV(indices.into()),
            SectionType::TangentTriangleSegment => {
                DecodedSegment::AppendTriangleTangent(indices.into())
            }
            _ => panic!("Unsupported section type {:?}", seg.seg_type),
        })
    } else {
        panic!("Unreachable condition reached!");
    }
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
fn search_for_sequential_regions(triangles: &[IndexType]) {
    /*
    let mut last = 0;
    let mut span_start = 0;
    let mut span_len = 0;
    let mut pos = 0;
    let mut spans:SmallVec<[(usize,usize);4]> = SmallVec::new();
    for index in triangles{
        if last + 1 == *index{
            if span_len == 0{
                span_start = pos;
            }
            span_len += 1;
        }
        else{
            if span_len > 4{
                spans.push((span_start,span_len));
                println!("span_start:{span_start:?},span_len:{span_len:?}");
            }
            span_len = 0;
        }
        last = *index;
        pos += 1;
    }
    if span_len > 4{
        spans.push((span_start,span_len));
        println!("span_start:{span_start:?},span_len:{span_len:?}");;
    }
    todo!();
    */
}
fn opt_tris(triangles: &[IndexType]) -> SmallVec<[&[IndexType]; 4]> {
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
fn opt_vertices(vertices: &[Vector3]) -> SmallVec<[&[Vector3]; 4]> {
    /*
    if vertices.len() < 16 {
        return smallvec![vertices];
    }*/
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
impl DecodedSegment {
    pub(crate) async fn optimize(self) -> SmallVec<[Self; 1]> {
        match self {
            Self::AppendTriangleVertex(triangles) => {
                search_for_sequential_regions(&triangles);
                let optimised = opt_tris(&triangles);
                let mut res = SmallVec::new();
                for seg in optimised {
                    res.push(Self::AppendTriangleVertex(seg.into()));
                }
                res
            }
            Self::AppendTriangleUV(triangles) => {
                let optimised = opt_tris(&triangles);
                let mut res = SmallVec::new();
                for seg in optimised {
                    res.push(Self::AppendTriangleUV(seg.into()));
                }
                res
            }
            Self::AppendTriangleNormal(triangles) => {
                let optimised = opt_tris(&triangles);
                let mut res = SmallVec::new();
                for seg in optimised {
                    res.push(Self::AppendTriangleNormal(seg.into()));
                }
                res
            }
            Self::AppendTriangleTangent(triangles) => {
                let optimised = opt_tris(&triangles);
                let mut res = SmallVec::new();
                for seg in optimised {
                    res.push(Self::AppendTriangleTangent(seg.into()));
                }
                res
            }
            Self::AppendVertex(vertices) => {
                let optimised = opt_vertices(&vertices);
                let mut res = SmallVec::new();
                for seg in optimised {
                    res.push(Self::AppendVertex(seg.into()));
                }
                res
            }
            _ => smallvec![self],
        }
    }
    pub(crate) async fn encode(
        self,
        prec: &TMFPrecisionInfo,
        ei: &EncodeInfo,
    ) -> Result<EncodedSegment, TMFExportError> {
        let mut data = vec![];
        let seg_type = match self {
            Self::AppendVertex(vertices) => {
                crate::vertices::save_tmf_vertices(
                    &vertices,
                    prec.vertex_precision,
                    &mut data,
                    ei.shortest_edge(),
                )?;
                SectionType::VertexSegment
            }
            Self::AppendTangent(tangents) => {
                crate::tangents::save_tangents(&tangents, prec.tangent_prec, &mut data)?;
                SectionType::TangentSegment
            }
            Self::AppendNormal(normals) => {
                crate::normals::save_normal_array(&normals, &mut data, prec.normal_precision)?;
                SectionType::NormalSegment
            }
            Self::AppendUV(uvs) => {
                crate::uv::save_uvs(&uvs, &mut data, prec.uv_prec)?;
                SectionType::UvSegment
            }
            Self::AppendTriangleVertex(triangles) => {
                let max_index = triangles.iter().max().unwrap_or(&0);
                crate::vertices::save_triangles(&triangles, (*max_index) as usize, &mut data)?;
                SectionType::VertexTriangleSegment
            }
            Self::AppendTriangleNormal(triangles) => {
                let max_index = triangles.iter().max().unwrap_or(&0);
                crate::vertices::save_triangles(&triangles, (*max_index) as usize, &mut data)?;
                SectionType::NormalTriangleSegment
            }
            Self::AppendTriangleUV(triangles) => {
                let max_index = triangles.iter().max().unwrap_or(&0);
                crate::vertices::save_triangles(&triangles, (*max_index) as usize, &mut data)?;
                SectionType::UvTriangleSegment
            }
            Self::AppendTriangleTangent(triangles) => {
                let max_index = triangles.iter().max().unwrap_or(&0);
                crate::vertices::save_triangles(&triangles, (*max_index) as usize, &mut data)?;
                SectionType::TangentTriangleSegment
            }
            Self::AppendCustom(custom_data) => custom_data.encode(&mut data)?,
            Self::Nothing => SectionType::Invalid,
        };
        Ok(EncodedSegment {
            seg_type,
            data: data.into(),
            compresion_type: CompressionType::None,
        })
    }
    pub(crate) async fn decode(
        seg: EncodedSegment,
        ctx: &crate::tmf_importer::TMFImportContext,
    ) -> Result<Self, TMFImportError> {
        match seg.seg_type {
            SectionType::Invalid => Ok(Self::Nothing),
            SectionType::VertexSegment => decode_vertex_seg(seg).await,
            SectionType::NormalSegment => decode_normal_seg(seg).await,
            SectionType::UvSegment => decode_uv_seg(seg).await,
            SectionType::TangentSegment => {
                async {
                    let tans = crate::tangents::read_tangents(&mut &seg.data[..])?;
                    Ok(DecodedSegment::AppendTangent(tans))
                }
                .await
            }
            SectionType::VertexTriangleSegment
            | SectionType::NormalTriangleSegment
            | SectionType::UvTriangleSegment
            | SectionType::ColorTriangleSegment
            | SectionType::TangentTriangleSegment => decode_triangle_seg(seg, ctx).await,
            SectionType::CustomIndexSegment | SectionType::CustomFloatSegment => {
                decode_custom_seg(seg, ctx).await
            }
            _ => todo!("Unhandled segement type {:?}", seg.seg_type),
        }
    }
    pub(crate) fn apply(&self, mesh: &mut TMFMesh) {
        match self {
            DecodedSegment::AppendVertex(verts) => mesh.append_vertices(verts),
            DecodedSegment::AppendNormal(norms) => mesh.append_normals(norms),
            DecodedSegment::AppendUV(uvs) => mesh.append_uvs(uvs),
            DecodedSegment::AppendTriangleVertex(vert_triangles) => {
                mesh.append_vertex_triangles(vert_triangles)
            }
            DecodedSegment::AppendTriangleNormal(norm_triangles) => {
                mesh.append_normal_triangles(norm_triangles)
            }
            DecodedSegment::AppendTriangleUV(uv_triangles) => {
                mesh.append_uv_triangles(uv_triangles)
            }
            DecodedSegment::AppendCustom(custom_data_seg) => {
                mesh.add_custom_data_seg(custom_data_seg.clone())
            }
            DecodedSegment::AppendTangent(tans) => mesh.append_tangents(tans),
            DecodedSegment::AppendTriangleTangent(tan_triangles) => {
                mesh.append_tangent_triangles(tan_triangles)
            }
            DecodedSegment::Nothing => (),
        }
    }
}
