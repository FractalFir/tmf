use crate::tmf_exporter::EncodeInfo;
use crate::tmf_importer::TMFImportContext;
use crate::unaligned_rw::{UnalignedRWMode, UnalignedReader};
use futures::executor::ThreadPool;
use crate::{
    CustomDataSegment, IndexType, TMFExportError, TMFImportError, TMFMesh, TMFPrecisionInfo,
    Vector2, Vector3, MAX_SEG_SIZE,
};
lazy_static::lazy_static!{
   pub(crate) static ref THREAD_POOL:ThreadPool = ThreadPool::new().expect("Could not create the thread pool!");
}
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
            15 => Self::CustomIndexSegment,
            16 => Self::CustomFloatSegment,
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
#[derive(Debug)]
pub(crate) enum DecodedSegment {
    Nothing,
    AppendVertex(Box<[Vector3]>),
    AppendNormal(Box<[Vector3]>),
    AppendUV(Box<[Vector2]>),
    AppendTriangleVertex(Box<[IndexType]>),
    AppendTriangleNormal(Box<[IndexType]>),
    AppendTriangleUV(Box<[IndexType]>),
    AppendCustom(CustomDataSegment),
}
pub(crate) struct EncodedSegment {
    seg_type: SectionType,
    compresion_type: CompressionType,
    data: Box<[u8]>,
}
impl EncodedSegment {
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
        let seg_type = ctx.stw().read(src)?;
        let data_length = ctx.slw().read(src)?;
        let compresion_type = {
            let mut tmp = [0];
            src.read_exact(&mut tmp)?;
            CompressionType::from_u8(tmp[0])?
        };
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
    let precision = {
        let mut tmp = [0];
        src.read_exact(&mut tmp)?;
        tmp[0]
    };
    let length = {
        let mut tmp = [0; std::mem::size_of::<u64>()];
        src.read_exact(&mut tmp)?;
        u64::from_le_bytes(tmp)
    };
    let min = ctx.read_traingle_min(&mut src)?;
    if length > MAX_SEG_SIZE as u64 {
        return Err(TMFImportError::SegmentTooLong);
    }
    *data = Vec::with_capacity(length as usize);
    let precision = UnalignedRWMode::precision_bits(precision);
    let mut reader = UnalignedReader::new(src);
    for _ in 0..length {
        data.push((reader.read_unaligned(precision)? + min) as IndexType);
    }
    Ok(())
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
impl DecodedSegment {
    pub(crate) async fn optimize(self) -> SmallVec<[Self; 1]> {
        match self {
            Self::AppendTriangleVertex(triangles) => {
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
            Self::AppendNormal(normals) => {
                crate::normals::save_normal_array(&normals, &mut data, prec.normal_precision)?;
                SectionType::NormalSegment
            }
            Self::AppendUV(uvs) => {
                crate::uv::save_uvs(&uvs, &mut data, prec.uv_prec)?;
                SectionType::UvSegment
            }
            Self::AppendTriangleVertex(triangles) => {
                let max_index = triangles.iter().max().unwrap_or(&1);
                crate::vertices::save_triangles(&triangles, (*max_index) as usize, &mut data)?;
                SectionType::VertexTriangleSegment
            }
            Self::AppendTriangleNormal(triangles) => {
                let max_index = triangles.iter().max().unwrap_or(&1);
                crate::vertices::save_triangles(&triangles, (*max_index) as usize, &mut data)?;
                SectionType::NormalTriangleSegment
            }
            Self::AppendTriangleUV(triangles) => {
                let max_index = triangles.iter().max().unwrap_or(&1);
                crate::vertices::save_triangles(&triangles, (*max_index) as usize, &mut data)?;
                SectionType::UvTriangleSegment
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
            DecodedSegment::Nothing => (),
        }
    }
}
