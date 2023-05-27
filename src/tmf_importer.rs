use crate::tmf::{CompressionType, SectionType};
use crate::tmf_exporter::EncodeInfo;
use crate::unaligned_rw::{UnalignedRWMode, UnalignedReader};
use crate::CustomDataSegment;
use crate::FloatType;
use crate::TMFExportError;
use crate::{
    IndexType, TMFImportError, TMFMesh, TMFPrecisionInfo, Vector2, Vector3, MAX_SEG_SIZE,
    TMF_MAJOR, TMF_MINOR,
};
use futures::{executor::block_on, future::join_all};
use smallvec::{smallvec, SmallVec};
enum SegLenWidth {
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
    fn read<R: std::io::Read>(&self, src: &mut R) -> std::io::Result<usize> {
        Ok(match self {
            Self::U32 => {
                let mut tmp = [0; std::mem::size_of::<u32>()];
                src.read_exact(&mut tmp)?;
                u32::from_le_bytes(tmp) as usize
            }
            Self::U64 => {
                let mut tmp = [0; std::mem::size_of::<u64>()];
                src.read_exact(&mut tmp)?;
                u64::from_le_bytes(tmp) as usize
            }
        })
    }
}
enum SegTypeWidth {
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
    fn read<R: std::io::Read>(&self, src: &mut R) -> std::io::Result<SectionType> {
        Ok(match self {
            Self::U8 => {
                let mut tmp = [0; std::mem::size_of::<u8>()];
                src.read_exact(&mut tmp)?;
                SectionType::from_u8(u8::from_le_bytes(tmp))
            }
            Self::U16 => {
                let mut tmp = [0; std::mem::size_of::<u16>()];
                src.read_exact(&mut tmp)?;
                SectionType::from_u16(u16::from_le_bytes(tmp))
            }
        })
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
    fn read<R: std::io::Read>(ctx: &TMFImportContext, src: &mut R) -> Result<Self, TMFImportError> {
        let seg_type = ctx.stw.read(src)?;
        let data_length = ctx.slw.read(src)? as usize;
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
async fn decode_custom_seg(seg: EncodedSegment,ctx:&crate::tmf_importer::TMFImportContext) -> Result<DecodedSegment, TMFImportError> {
    if matches!(
        seg.seg_type,
        SectionType::CustomIndexSegment | SectionType::CustomFloatSegment
    ) {
        let mut data: &[u8] = &seg.data[..];
        Ok(DecodedSegment::AppendCustom(CustomDataSegment::read(
            &mut data,
            seg.seg_type,
            ctx
        )?))
    } else {
        panic!("Unreachable condition reached!");
    }
}
fn read_default_triangles<R: std::io::Read>(
    mut src: R,
    data: &mut Vec<IndexType>,
    ctx:&crate::tmf_importer::TMFImportContext
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
    for _ in 0..length{
        data.push((reader.read_unaligned(precision)? + min) as IndexType);
    }
    Ok(())
}
async fn decode_triangle_seg(mut seg: EncodedSegment,ctx:&crate::tmf_importer::TMFImportContext) -> Result<DecodedSegment, TMFImportError> {
    if seg.seg_type.is_triangle() {
        let mut data: &[u8] = &seg.data[..];
        let mut indices = Vec::new();
        match seg.compresion_type {
            CompressionType::None => read_default_triangles(data, &mut indices,ctx)?,
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
fn calc_spilt_score(len: usize, delta_span:(IndexType,IndexType)) -> isize {
    //return isize::MIN;
    use crate::vertices:: calc_prec;
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
    for index in 0..triangles.len() {
        max_index = max_index.max(triangles[index]);
        let score = calc_spilt_score(index, (*total_max,max_index));
        if score > best_score {
            best_index = index;
            best_score = score;
        }
    }
    if best_score > 0{
        let mut res = SmallVec::new();
        let (s1,s2) = triangles.split_at(best_index);
        let r_1 = opt_tris(s1);
        for seg in r_1{
            res.push(seg);
        }
        let r_2 = opt_tris(s2);
        for seg in r_2{
            res.push(seg);
        }
        return res;
    }
    let mut best_score = isize::MIN;
    let mut best_index = usize::MIN;
    let mut min_index = IndexType::MAX;
    let total_min = triangles.iter().min().unwrap_or(&1);
    for index in 0..triangles.len() {
        min_index = min_index.min(triangles[index]);
        let score = calc_spilt_score(index, (min_index, *total_min));
        if score > best_score {
            best_index = index;
            best_score = score;
        }
    }
    if best_score > 0{
        let mut res = SmallVec::new();
        let (s1,s2) = triangles.split_at(best_index);
        let r_1 = opt_tris(s1);
        for seg in r_1{
            res.push(seg);
        }
        let r_2 = opt_tris(s2);
        for seg in r_2{
            res.push(seg);
        }
        res
    }
    else{
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
                crate::uv::save_uvs(
                    &uvs,
                    &mut data,
                    prec.uv_prec,
                )?;
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
    async fn decode(seg: EncodedSegment,ctx:&crate::tmf_importer::TMFImportContext) -> Result<Self, TMFImportError> {
        match seg.seg_type {
            SectionType::Invalid => Ok(Self::Nothing),
            SectionType::VertexSegment => decode_vertex_seg(seg).await,
            SectionType::NormalSegment => decode_normal_seg(seg).await,
            SectionType::UvSegment => decode_uv_seg(seg).await,
            SectionType::VertexTriangleSegment
            | SectionType::NormalTriangleSegment
            | SectionType::UvTriangleSegment
            | SectionType::ColorTriangleSegment
            | SectionType::TangentTriangleSegment => decode_triangle_seg(seg,ctx).await,
            SectionType::CustomIndexSegment | SectionType::CustomFloatSegment => {
                decode_custom_seg(seg,ctx).await
            }
            _ => todo!("Unhandled segement type {:?}", seg.seg_type),
        }
    }
    fn apply(&self, mesh: &mut TMFMesh) {
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
            _ => todo!("Can't apply decoded segment {self:?}"),
        }
    }
}
pub(crate) struct TMFImportContext {
    slw: SegLenWidth,
    stw: SegTypeWidth,
    should_read_min_index:bool,
    meshes: Vec<TMFMesh>,
}
struct TMFHeader {
    major: u16,
    minor: u16,
    min_major: u16,
    min_minor: u16,
}
pub(crate) fn read_string<R: std::io::Read>(src: &mut R) -> std::io::Result<String> {
    let byte_len = {
        let mut tmp = [0; std::mem::size_of::<u16>()];
        src.read_exact(&mut tmp)?;
        u16::from_le_bytes(tmp)
    };
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
async fn read_tmf_header<R: std::io::Read>(src: &mut R) -> Result<TMFHeader, TMFImportError> {
    let mut magic = [0; 3];
    src.read_exact(&mut magic)?;
    if magic != *b"TMF" {
        return Err(TMFImportError::NotTMFFile);
    }
    let major = {
        let mut tmp = [0; std::mem::size_of::<u16>()];
        src.read_exact(&mut tmp)?;
        u16::from_le_bytes(tmp)
    };
    let minor = {
        let mut tmp = [0; std::mem::size_of::<u16>()];
        src.read_exact(&mut tmp)?;
        u16::from_le_bytes(tmp)
    };
    // Minimum version of reader required to read
    let min_major = {
        let mut tmp = [0; std::mem::size_of::<u16>()];
        src.read_exact(&mut tmp)?;
        u16::from_le_bytes(tmp)
    };
    let min_minor = {
        let mut tmp = [0; std::mem::size_of::<u16>()];
        src.read_exact(&mut tmp)?;
        u16::from_le_bytes(tmp)
    };
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
    pub(crate) fn read_traingle_min<R:std::io::Read>(&self,src:&mut R)->std::io::Result<u64>{
        if self.should_read_min_index{
            let mut tmp = [0; std::mem::size_of::<u64>()];
            src.read_exact(&mut tmp)?;
            Ok(u64::from_le_bytes(tmp))
        }
        else{
            Ok(0)
        }
    }
    fn init_header(hdr: TMFHeader) -> Self {
        let slw = SegLenWidth::from_header(&hdr);
        let stw = SegTypeWidth::from_header(&hdr);
        Self {
            slw,
            stw,
            meshes: Vec::new(),
            should_read_min_index: (hdr.min_minor > 1)
        }
    }
    async fn import_mesh<R: std::io::Read>(
        &self,
        mut src: R,
        ctx:&crate::tmf_importer::TMFImportContext
    ) -> Result<(TMFMesh, String), TMFImportError> {
        let name = read_string(&mut src)?;
        let segment_count = {
            let mut tmp = [0; std::mem::size_of::<u16>()];
            src.read_exact(&mut tmp)?;
            u16::from_le_bytes(tmp)
        }; //self.slw.read(&mut src)?;
        let mut decoded_segs = Vec::with_capacity(segment_count as usize);
        for _ in 0..segment_count {
            let encoded = EncodedSegment::read(self, &mut src)?;
            decoded_segs.push(DecodedSegment::decode(encoded,ctx));
        }
        let mut res = TMFMesh::empty();
        join_all(decoded_segs)
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?
            .iter()
            .for_each(|seg| {
                seg.apply(&mut res);
            });
        Ok((res, name))
    }
    pub(crate) async fn import<R: std::io::Read>(
        mut src: R,
    ) -> Result<Vec<(TMFMesh, String)>, TMFImportError> {
        let header = read_tmf_header(&mut src).await?;
        let res = Self::init_header(header);
        let mesh_count = {
            let mut tmp = [0; std::mem::size_of::<u32>()];
            src.read_exact(&mut tmp)?;
            u32::from_le_bytes(tmp)
        };
        let mut meshes = Vec::with_capacity((u16::MAX as usize).min(mesh_count as usize));
        for _ in 0..mesh_count {
            meshes.push(res.import_mesh(&mut src,&res).await?);
        }
        Ok(meshes)
    }
}
pub(crate) fn import_sync<R: std::io::Read>(
    mut src: R,
) -> Result<Vec<(TMFMesh, String)>, TMFImportError> {
    futures::executor::block_on(TMFImportContext::import(src))
}
#[cfg(test)]
fn init_test_env() {
    std::fs::create_dir_all("target/test_res").unwrap();
}
#[cfg(test)]
#[test]
fn test() {
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
    let imported = futures::executor::block_on(TMFImportContext::import(&out[..])).unwrap();
}
#[cfg(test)]
#[test]
fn test_triangles_opt(){
    let mut tmp = Vec::with_capacity(1000_000);
    for i in 0..1000{
        tmp.push(i);
    }
    let tris = DecodedSegment::AppendTriangleVertex(tmp.into());
    let tris = block_on(tris.optimize());
    let tris:Vec<EncodedSegment> = tris.into_iter().map(
        |seg| {
            block_on(seg.encode(&TMFPrecisionInfo::default(),&EncodeInfo::default())).unwrap()
        }
    ).collect();
    let ctx = TMFImportContext::init_header(TMFHeader{
        major:crate::TMF_MAJOR,minor:crate::TMF_MINOR,min_major:crate::MIN_TMF_MAJOR,min_minor:crate::MIN_TMF_MINOR
    });
    let tris:Vec<DecodedSegment> = tris.into_iter().map(
        |seg| {
            let seg:DecodedSegment = block_on(DecodedSegment::decode(seg,&ctx)).unwrap();
            seg
        }
    ).collect();
    println!("{tris:?}");
    let mut curr = 0;
    for seg in tris.iter(){
        let values = if let DecodedSegment::AppendTriangleVertex(vals) = seg{vals}else{panic!()};
        for value in values.iter(){
            assert_eq!(*value,curr);
            curr += 1;
        }
    }
}