use crate::{TMFImportError, MAX_SEG_SIZE};
use std::io::Write;
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
