use crate::tmf::{CompressionType, SectionType};
use crate::{
    IndexType, TMFImportError, TMFMesh, TMFPrecisionInfo, Vector2, Vector3, TMF_MAJOR, TMF_MINOR,
};
use futures::executor::block_on;
use futures::future::join_all;
enum SegLenWidth {
    U32,
    U64,
}
impl SegLenWidth {
    fn from_header(header: &TMFHeader) -> Self {
        if header.min_minor >= 2 {
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
        if header.min_minor >= 2 {
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
enum DecodedSegment {
    Nothing,
    AppendVertex(Box<[Vector3]>),
    AppendNormal(Box<[Vector3]>),
    AppendUV(Box<[Vector2]>),
    AppendTriangleVertex(Box<[IndexType]>),
    AppendTriangleNormal(Box<[IndexType]>),
    AppendTriangleUV(Box<[IndexType]>),
}
struct EncodedSegment {
    seg_type: SectionType,
    compresion_type: CompressionType,
    data: Box<[u8]>,
}
impl EncodedSegment {
    fn read<R: std::io::Read>(ctx: &TMFImportContext, src: &mut R) -> Result<Self, TMFImportError> {
        let seg_type = ctx.stw.read(src)?;
        let data_length = {
            let mut tmp = [0; std::mem::size_of::<u64>()];
            src.read_exact(&mut tmp)?;
            u64::from_le_bytes(tmp)
        } as usize;
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
async fn decode_vertex_seg(mut seg: EncodedSegment) -> Result<DecodedSegment, TMFImportError> {
    if SectionType::VertexSegment == seg.seg_type {
        let mut data: &[u8] = &seg.data[..];
        Ok(DecodedSegment::AppendVertex(
            crate::vertices::read_tmf_vertices(&mut data)?,
        ))
    } else {
        panic!("Unreachable condition reached!");
    }
}
impl DecodedSegment {
    async fn decode(seg: EncodedSegment) -> Result<Self, TMFImportError> {
        match seg.seg_type {
            SectionType::Invalid => Ok(Self::Nothing),
            SectionType::VertexSegment => decode_vertex_seg(seg).await,
            _ => todo!("Unhandled sgement type {:?}", seg.seg_type),
        }
    }
}
struct TMFImportContext {
    slw: SegLenWidth,
    stw: SegTypeWidth,
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
    fn init_header(hdr: TMFHeader) -> Self {
        let slw = SegLenWidth::from_header(&hdr);
        let stw = SegTypeWidth::from_header(&hdr);
        Self {
            slw,
            stw,
            meshes: Vec::new(),
        }
    }
    async fn import_mesh<R: std::io::Read>(&self, mut src: R) -> Result<TMFMesh, TMFImportError> {
        let mut res = TMFMesh::empty();
        let name = read_string(&mut src)?;
        let segment_count = {
            let mut tmp = [0; std::mem::size_of::<u16>()];
            src.read_exact(&mut tmp)?;
            u16::from_le_bytes(tmp)
        }; //self.slw.read(&mut src)?;
        let mut decoded_segs = Vec::with_capacity(segment_count as usize);
        for _ in 0..segment_count {
            let encoded = EncodedSegment::read(self, &mut src)?;
            decoded_segs.push(DecodedSegment::decode(encoded));
        }
        let decoded_segs = join_all(decoded_segs).await;
        todo!("{name} {segment_count} {decoded_segs:?}");
    }
    async fn import<R: std::io::Read>(mut src: R) -> Result<TMFImportContext, TMFImportError> {
        let header = read_tmf_header(&mut src).await?;
        let res = Self::init_header(header);
        let mesh_count = {
            let mut tmp = [0; std::mem::size_of::<u32>()];
            src.read_exact(&mut tmp)?;
            u32::from_le_bytes(tmp)
        };
        let mut meshes = Vec::with_capacity((u16::MAX as usize).min(mesh_count as usize));
        println!("{mesh_count}");
        for _ in 0..mesh_count {
            meshes.push(res.import_mesh(&mut src).await?);
        }
        todo!();
    }
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
