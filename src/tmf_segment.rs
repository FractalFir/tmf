use crate::{TMFMesh, Vector3};
#[repr(u16)]
#[derive(Debug)]
pub(crate) enum SectionType {
    Invalid = 0,
    VertexSegment = 1,
    VertexTriangleSegment = 2,
    NormalSegment = 3,
    NormalTriangleSegment = 4,
    UvSegment = 5,
    UvTriangleSegment = 6,
    MaterialInfo = 7,
    Materialtriangles = 8,
}
impl SectionType {
    pub fn from_u16(input: u16) -> Self {
        match input {
            1 => Self::VertexSegment,
            2 => Self::VertexTriangleSegment,
            3 => Self::NormalSegment,
            4 => Self::NormalTriangleSegment,
            5 => Self::UvSegment,
            6 => Self::UvTriangleSegment,
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
    pub fn from_u8(input: u8) -> Self {
        match input {
            0 => Self::None,
            1 => Self::Ommited,
            2 => Self::UnalignedLZZ,
            _ => panic!("Unknow CompressionType {input}"),
        }
    }
}
struct EncodedSegment {
    seg_type: SectionType,
    compresion_type: CompressionType,
    seg_data: Vec<u8>,
}
enum DecodedSegement {
    VertexSegment(Vec<Vector3>),
}
impl DecodedSegement {
    fn encode(self) -> EncodedSegment {
        todo!("Not working yet!");
    }
    fn apply(self, mesh: &mut TMFMesh) {
        match self {
            DecodedSegement::VertexSegment(vertices) => {
                /*match mesh.get_vertices(){
                    Some(vertices)=>
                }*/
                todo!();
            }
        }
    }
}
