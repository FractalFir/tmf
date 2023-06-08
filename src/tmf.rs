use crate::read_extension::ReadExt;
use crate::tmf_exporter::{opt_tris, opt_vertices, EncodeInfo};
use crate::tmf_importer::{
    decode_custom_seg, decode_normal_seg, decode_triangle_seg, decode_uv_seg, decode_vertex_seg,
    TMFImportContext,
};

use crate::{
    CustomDataSegment, IndexType, TMFExportError, TMFImportError, TMFMesh, TMFPrecisionInfo,
    Tangent, Vector2, Vector3,
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
    SharedTriangleSegment = 25,
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
                | Self::SharedTriangleSegment
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
            25 => Self::SharedTriangleSegment,
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
#[derive(Debug, Clone)]
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
    SharedTriangleSegment(SharedSegmentKind, Box<[IndexType]>),
    AppendCustom(CustomDataSegment),
}
pub(crate) struct EncodedSegment {
    seg_type: SectionType,
    compresion_type: CompressionType,
    data: Box<[u8]>,
}
impl EncodedSegment {
    pub(crate) fn data(&self) -> &[u8] {
        &self.data
    }
    pub(crate) fn compresion_type(&self) -> CompressionType {
        self.compresion_type
    }
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
#[derive(Default, Debug, Copy, Clone)]
pub(crate) struct SharedSegmentKind {
    mask: u8,
}
impl SharedSegmentKind {
    fn set_vertex(&mut self) {
        self.mask |= 0x1;
    }
    fn get_vertex(&self) -> bool {
        self.mask & 0x1 != 0
    }
    fn set_normal(&mut self) {
        self.mask |= 0x2;
    }
    fn get_normal(&self) -> bool {
        self.mask & 0x2 != 0
    }
    fn set_uv(&mut self) {
        self.mask |= 0x4;
    }
    fn get_uv(&self) -> bool {
        self.mask & 0x4 != 0
    }
    fn combine(self, other: Self) -> Self {
        Self {
            mask: self.mask | other.mask,
        }
    }
    fn mask(&self) -> u8 {
        self.mask
    }
    fn from_mask(mask: u8) -> Self {
        Self { mask }
    }
}
impl std::fmt::Display for SharedSegmentKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{{SharedSegMask:0x{:x} ->", self.mask)?;
        if self.get_vertex() {
            write!(f, "VertexTriangle")?;
        }
        if self.get_normal() {
            write!(f, "NormalTriangle")?;
        }
        if self.get_uv() {
            write!(f, "UVTriangle")?;
        }
        write!(f, "}}")
    }
}
impl DecodedSegment {
    pub(crate) fn merge(&mut self, other: &mut Self) {
        if other.as_triangles().is_none() {
            return;
        }
        match self {
            Self::SharedTriangleSegment(kind, indices) => {
                if &indices[..] != other.as_triangles().unwrap() {
                    return;
                }
                let combined = kind.combine(other.shared_kind());
                *self = Self::SharedTriangleSegment(combined, indices.clone());
                *other = Self::Nothing;
            }
            Self::AppendTriangleVertex(indices) => {
                if &indices[..] != other.as_triangles().unwrap() {
                    return;
                }
                let kind = {
                    let mut kind = SharedSegmentKind::default();
                    kind.set_vertex();
                    kind
                };
                let combined = kind.combine(other.shared_kind());
                *self = Self::SharedTriangleSegment(combined, indices.clone());
                *other = Self::Nothing;
            }
            Self::AppendTriangleNormal(indices) => {
                if &indices[..] != other.as_triangles().unwrap() {
                    return;
                }
                let kind = {
                    let mut kind = SharedSegmentKind::default();
                    kind.set_normal();
                    kind
                };
                let combined = kind.combine(other.shared_kind());
                *self = Self::SharedTriangleSegment(combined, indices.clone());
                *other = Self::Nothing;
            }
            Self::AppendTriangleUV(indices) => {
                if &indices[..] != other.as_triangles().unwrap() {
                    return;
                }
                let kind = {
                    let mut kind = SharedSegmentKind::default();
                    kind.set_uv();
                    kind
                };
                let combined = kind.combine(other.shared_kind());
                *self = Self::SharedTriangleSegment(combined, indices.clone());
                *other = Self::Nothing;
            }
            _ => (),
        }
    }
    fn as_triangles(&self) -> Option<&[IndexType]> {
        match self {
            Self::AppendTriangleVertex(indices) => Some(indices),
            Self::AppendTriangleNormal(indices) => Some(indices),
            _ => None,
        }
    }
    fn shared_kind(&self) -> SharedSegmentKind {
        match self {
            Self::AppendTriangleVertex(_) => {
                let mut kind = SharedSegmentKind::default();
                kind.set_vertex();
                kind
            }
            Self::AppendTriangleNormal(_) => {
                let mut kind = SharedSegmentKind::default();
                kind.set_normal();
                kind
            }
            _ => todo!("{:?}", self),
        }
    }
    pub(crate) fn is_something(&self) -> bool {
        !matches!(self, Self::Nothing)
    }
    pub(crate) async fn optimize(self) -> SmallVec<[Self; 1]> {
        match self {
            Self::AppendTriangleVertex(triangles) => {
                //search_for_sequential_regions(&triangles);
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
            Self::SharedTriangleSegment(kind, triangles) => {
                let max_index = triangles.iter().max().unwrap_or(&0);
                data.push(kind.mask());
                crate::vertices::save_triangles(&triangles, (*max_index) as usize, &mut data)?;
                SectionType::SharedTriangleSegment
            }
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
        //println!("Decoding segment of type:{:?}!",seg.seg_type());
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
            | SectionType::TangentTriangleSegment => {
                decode_triangle_seg(seg, ctx).await},
            SectionType::CustomIndexSegment | SectionType::CustomFloatSegment => {
                decode_custom_seg(seg, ctx).await
            }
            SectionType::SharedTriangleSegment => {
                let kind = SharedSegmentKind::from_mask(seg.data[0]);
                let data: &[u8] = &seg.data()[1..];
                let mut indices = Vec::new();
                match seg.compresion_type() {
                    CompressionType::None => {
                        crate::tmf_importer::read_default_triangles(data, &mut indices, ctx)
                            .unwrap()
                    }
                    _ => panic!("Shared segments may only be uncompressed in TMF 0.2"),
                }
                Ok(Self::SharedTriangleSegment(kind, indices.into()))
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
            DecodedSegment::SharedTriangleSegment(kind, indices) => {
                //println!("kind:{kind}");
                if kind.get_vertex() {
                    mesh.append_vertex_triangles(indices);
                }
                if kind.get_normal() {
                    mesh.append_normal_triangles(indices);
                }
                if kind.get_uv() {
                    mesh.append_uv_triangles(indices);
                }
            }
        }
    }
}
