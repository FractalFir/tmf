use crate::read_extension::ReadExt;
use crate::tmf::CompressionType;
use crate::tmf::{DecodedSegment, EncodedSegment, SectionType};
use crate::unaligned_rw::UnalignedRWMode;
use crate::unaligned_rw::UnalignedReader;
use crate::CustomDataSegment;
use crate::IndexType;
use crate::MAX_SEG_SIZE;
use crate::{TMFImportError, TMFMesh, TMF_MAJOR, TMF_MINOR};
use futures::future::join_all;
use std::io::Read;
#[derive(Clone, Copy)]
pub(crate) enum SegLenWidth {
    U32,
    U64,
}
impl SegLenWidth {
    fn from_header(header: &TMFHeader) -> Self {
        if header.min_minor > 1 {
            Self::U32
        } else {
            Self::U64
        }
    }
    pub(crate) fn read<R: Read>(&self, src: &mut R) -> std::io::Result<usize> {
        Ok(match self {
            Self::U32 => src.read_u32()? as usize,
            Self::U64 => src.read_u64()? as usize,
        })
    }
}
#[derive(Clone, Copy)]
pub(crate) enum SegTypeWidth {
    U16,
    U8,
}
impl SegTypeWidth {
    fn from_header(header: &TMFHeader) -> Self {
        if header.min_minor > 1 {
            Self::U8
        } else {
            Self::U16
        }
    }
    pub(crate) fn read<R: Read>(&self, src: &mut R) -> std::io::Result<SectionType> {
        Ok(match self {
            Self::U8 => SectionType::from_u8(src.read_u8()?),
            Self::U16 => SectionType::from_u16(src.read_u16()?),
        })
    }
}
#[derive(Clone)]
pub(crate) struct TMFImportContext {
    segment_length_width: SegLenWidth,
    segment_type_width: SegTypeWidth,
    should_read_min_index: bool,
}
// While some of those fileds are not read yet, they may be relevant in the future.
#[allow(dead_code)]
struct TMFHeader {
    major: u16,
    minor: u16,
    min_major: u16,
    min_minor: u16,
}
pub(crate) fn read_string<R: Read>(src: &mut R) -> std::io::Result<String> {
    let byte_len = src.read_u16()?;
    let mut bytes = vec![0; byte_len as usize];
    src.read_exact(&mut bytes)?;
    match std::str::from_utf8(&bytes) {
        Ok(string) => Ok(string.to_owned()),
        Err(_) => Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Mesh name not valid UTF-8",
        )),
    }
}
async fn read_tmf_header<R: Read>(src: &mut R) -> Result<TMFHeader, TMFImportError> {
    let mut magic = [0; 3];
    src.read_exact(&mut magic)?;
    if magic != *b"TMF" {
        return Err(TMFImportError::NotTMFFile);
    }
    let major = src.read_u16()?;
    let minor = src.read_u16()?;
    // Minimum version of reader required to read
    let min_major = src.read_u16()?;
    let min_minor = src.read_u16()?;

    if min_major > TMF_MAJOR || (min_major == TMF_MAJOR && min_minor > TMF_MINOR) {
        Err(TMFImportError::NewerVersionRequired)
    } else {
        Ok(TMFHeader {
            major,
            minor,
            min_major,
            min_minor,
        })
    }
}
impl TMFImportContext {
    pub(crate) fn segment_type_width(&self) -> &SegTypeWidth {
        &self.segment_type_width
    }
    pub(crate) fn segment_length_width(&self) -> &SegLenWidth {
        &self.segment_length_width
    }
    pub(crate) fn read_traingle_min<R: Read>(&self, src: &mut R) -> std::io::Result<u64> {
        if self.should_read_min_index {
            src.read_u64()
        } else {
            Ok(0)
        }
    }
    fn init_header(hdr: TMFHeader) -> Self {
        Self {
            segment_length_width: SegLenWidth::from_header(&hdr),
            segment_type_width: SegTypeWidth::from_header(&hdr),
            should_read_min_index: (hdr.min_minor > 1),
        }
    }
    async fn import_mesh<R: Read>(
        &self,
        mut src: R,
        ctx: &Self,
    ) -> Result<(TMFMesh, String), TMFImportError> {
        let name = read_string(&mut src)?;
        let segment_count = src.read_u16()?;
        //self.segment_length_width.read(&mut src)?;
        let mut decoded_segs = Vec::with_capacity(segment_count as usize);
        for _ in 0..segment_count {
            let encoded = EncodedSegment::read(self, &mut src)?;
            //println!("encoded_type:{:?}",encoded.seg_type());
            let ctx = ctx.clone();
            let decoded = async move { DecodedSegment::decode(encoded, &ctx).await };

            #[cfg(feature = "tokio_runtime")]
            let decoded = { tokio::task::spawn(decoded) };

            decoded_segs.push(decoded);
        }
        let mut res = TMFMesh::empty();
        let joined = join_all(decoded_segs).await;
        #[cfg(feature = "tokio_runtime")]
        let joined = { joined.into_iter().collect::<Result<Vec<_>, _>>().unwrap() };
        joined
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?
            .iter()
            .for_each(|seg| {
                seg.apply(&mut res);
            });
        Ok((res, name))
    }
    async fn analize_mesh<R: Read>(&self, mut src: R, _ctx: &Self) -> Result<(), TMFImportError> {
        let _name = read_string(&mut src)?;
        let segment_count = src.read_u16()?;
        let mut results = [0; 256];
        for _ in 0..segment_count {
            let encoded = EncodedSegment::read(self, &mut src)?;
            results[encoded.seg_type() as u8 as usize] += encoded.seg_length();
        }
        let mut res = Vec::new();
        let mut total = 0;
        for i in 0..results.len() {
            let byte_len = results[i];
            total += byte_len;
            if byte_len != 0 {
                res.push((format!("{:?}", SectionType::from_u8(i as u8)), byte_len));
            }
        }
        res.sort_by(|a, b| a.1.cmp(&b.1));
        println!("res:{res:?}, total_len:{total}");
        Ok(())
    }
    pub(crate) async fn import<R: Read>(
        mut src: R,
    ) -> Result<Vec<(TMFMesh, String)>, TMFImportError> {
        let header = read_tmf_header(&mut src).await?;
        let res = Self::init_header(header);
        let mesh_count = src.read_u32()?;
        let mut meshes = Vec::with_capacity((u16::MAX as usize).min(mesh_count as usize));
        for _ in 0..mesh_count {
            meshes.push(res.import_mesh(&mut src, &res).await?);
        }
        Ok(meshes)
    }
    pub(crate) async fn analize<R: Read>(mut src: R) -> Result<(), TMFImportError> {
        let header = read_tmf_header(&mut src).await?;
        let res = Self::init_header(header);
        let mesh_count = src.read_u32()?;
        for _ in 0..mesh_count {
            res.analize_mesh(&mut src, &res).await?;
        }
        Ok(())
    }
}
pub(crate) fn import_sync<R: std::io::Read>(
    src: R,
) -> Result<Vec<(TMFMesh, String)>, TMFImportError> {
    runtime_agnostic_block_on!(TMFImportContext::import(src))
}
pub(crate) async fn decode_vertex_seg(
    seg: EncodedSegment,
) -> Result<DecodedSegment, TMFImportError> {
    if SectionType::VertexSegment == seg.seg_type() {
        let mut data: &[u8] = seg.data();
        Ok(DecodedSegment::AppendVertex(
            crate::vertices::read_tmf_vertices(&mut data)?,
        ))
    } else {
        panic!("Unreachable condition reached!");
    }
}
pub(crate) async fn decode_uv_seg(seg: EncodedSegment) -> Result<DecodedSegment, TMFImportError> {
    if SectionType::UvSegment == seg.seg_type() {
        let mut data: &[u8] = seg.data();
        Ok(DecodedSegment::AppendUV(crate::uv::read_uvs(&mut data)?))
    } else {
        panic!("Unreachable condition reached!");
    }
}
pub(crate) async fn decode_normal_seg(
    seg: EncodedSegment,
) -> Result<DecodedSegment, TMFImportError> {
    if SectionType::NormalSegment == seg.seg_type() {
        let mut data: &[u8] = seg.data();
        Ok(DecodedSegment::AppendNormal(
            crate::normals::read_normal_array(&mut data)?,
        ))
    } else {
        panic!("Unreachable condition reached!");
    }
}
pub(crate) async fn decode_custom_seg(
    seg: EncodedSegment,
    ctx: &crate::tmf_importer::TMFImportContext,
) -> Result<DecodedSegment, TMFImportError> {
    if matches!(
        seg.seg_type(),
        SectionType::CustomIndexSegment | SectionType::CustomFloatSegment
    ) {
        let mut data: &[u8] = seg.data();
        Ok(DecodedSegment::AppendCustom(CustomDataSegment::read(
            &mut data,
            seg.seg_type(),
            ctx,
        )?))
    } else {
        panic!("Unreachable condition reached!");
    }
}
pub(crate) fn read_default_triangles<R: std::io::Read>(
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
pub(crate) async fn decode_triangle_seg(
    seg: EncodedSegment,
    ctx: &crate::tmf_importer::TMFImportContext,
) -> Result<DecodedSegment, TMFImportError> {
    if seg.seg_type().is_triangle() {
        let data: &[u8] = seg.data();
        let mut indices = Vec::new();
        match seg.compresion_type() {
            CompressionType::None => read_default_triangles(data, &mut indices, ctx)?,
            CompressionType::Sequence => read_triangle_sequence(data, &mut indices)?,
            CompressionType::Ommited => panic!("New decoder does not support ommited segment!"),
            CompressionType::UnalignedLZZ => panic!("Unaligned lzz not supported yet!"),
        };
        Ok(match seg.seg_type() {
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
            _ => panic!("Unsupported section type {:?}", seg.seg_type()),
        })
    } else {
        panic!("Unreachable condition reached!");
    }
}
#[cfg(test)]
fn init_test_env() {
    std::fs::create_dir_all("target/test_res").unwrap();
}
#[cfg(test)]
#[test]
fn test() {
    use crate::TMFPrecisionInfo;
    init_test_env();
    let mut file = std::fs::File::open("testing/susan.obj").unwrap();
    let (tmf_mesh, name) = TMFMesh::read_from_obj_one(&mut file).unwrap();
    tmf_mesh.verify().unwrap();
    assert!(name == "Suzanne", "Name should be Suzanne but is {name}");
    let prec = TMFPrecisionInfo::default();
    let mut out = Vec::new();
    {
        tmf_mesh.write_tmf_one(&mut out, &prec, name).unwrap();
    }
    let _imported = runtime_agnostic_block_on!(TMFImportContext::import(&out[..])).unwrap();
}
#[cfg(test)]
#[test]
fn test_triangles_opt() {
    use crate::tmf_exporter::EncodeInfo;
    use crate::TMFPrecisionInfo;
    let mut tmp = Vec::with_capacity(1_000_000);
    for i in 0..1000 {
        tmp.push(i);
    }
    let tris = DecodedSegment::AppendTriangleVertex(tmp.into());
    let tris = runtime_agnostic_block_on!(tris.optimize());
    let tris: Vec<EncodedSegment> = tris
        .into_iter()
        .map(|seg| {
            runtime_agnostic_block_on!(
                seg.encode(&TMFPrecisionInfo::default(), &EncodeInfo::default())
            )
            .unwrap()
        })
        .collect();
    let ctx = TMFImportContext::init_header(TMFHeader {
        major: crate::TMF_MAJOR,
        minor: crate::TMF_MINOR,
        min_major: crate::MIN_TMF_MAJOR,
        min_minor: crate::MIN_TMF_MINOR,
    });
    let tris: Vec<DecodedSegment> = tris
        .into_iter()
        .map(|seg| {
            let seg: DecodedSegment =
                runtime_agnostic_block_on!(DecodedSegment::decode(seg, &ctx)).unwrap();
            seg
        })
        .collect();
    let mut curr = 0;
    for seg in tris.iter() {
        let values = if let DecodedSegment::AppendTriangleVertex(vals) = seg {
            vals
        } else {
            panic!()
        };
        for value in values.iter() {
            assert_eq!(*value, curr);
            curr += 1;
        }
    }
}
