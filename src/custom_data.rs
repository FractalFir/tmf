use crate::tmf::SectionType;
use crate::FloatType;
use crate::IndexType;
#[derive(Clone)]
pub struct CustomDataSegment {
    name: [u8; u8::MAX as usize],
    name_len: u8,
    data: CustomData,
}
#[derive(Debug, Copy, Clone)]
pub enum DataSegmentError {
    NoName,
    NameTooLong,
}
impl CustomDataSegment {
    fn new_raw(data: CustomData, raw: [u8; u8::MAX as usize], name_len: u8) -> Self {
        Self {
            name: raw,
            name_len,
            data,
        }
    }
    pub fn custom_data(&self) -> &CustomData {
        &self.data
    }
    pub(crate) fn name_len(&self) -> u8 {
        self.name_len
    }
    pub(crate) fn name_bytes(&self) -> &[u8] {
        &self.name
    }
    pub fn new(data: CustomData, name: &str) -> Result<Self, DataSegmentError> {
        let bytes = name.as_bytes();
        let len = name.len();
        if len > u8::MAX as usize {
            return Err(DataSegmentError::NameTooLong);
        } else if len == 0 {
            return Err(DataSegmentError::NoName);
        }
        let name_len = len as u8;
        let mut name = [0; u8::MAX as usize];
        for index in 0..bytes.len() {
            name[index] = bytes[index];
        }
        Ok(Self::new_raw(data, name, name_len))
    }
}
#[derive(Clone)]
pub enum CustomData {
    CustomIndex(Box<[IndexType]>, usize),
    //CustomFloat(Box<[FloatType]>),
}
impl CustomData {
    /// Returns the index data if custom segment is an index segment. Returns the index array and max index.
    pub fn is_index(&self) -> Option<(&[IndexType], usize)> {
        match self {
            Self::CustomIndex(array, max_index) => Some((&array, *max_index)),
            _ => None,
        }
    }
    fn write<W: std::io::Write>(&self, target: &mut W) -> std::io::Result<()> {
        match self {
            Self::CustomIndex(data, max_index) => {
                crate::vertices::save_triangles(data, *max_index, target)
            }
        }?;
        Ok(())
    }
    fn section_type(&self) -> SectionType {
        match self {
            Self::CustomIndex(_, _) => SectionType::CustomIndexSegment,
        }
    }
    fn new_index(indices: &[IndexType], max_index: Option<usize>) -> Self {
        let max_index = match max_index {
            Some(max_index) => max_index,
            None => indices.iter().max().copied().unwrap_or(0) as usize,
        };
        Self::CustomIndex(indices.into(), max_index)
    }
}
impl From<&[IndexType]> for CustomData {
    fn from(indices: &[IndexType]) -> Self {
        Self::new_index(indices.into(), None)
    }
}
impl CustomDataSegment {
    pub(crate) fn write<W: std::io::Write>(&self, target: &mut W) -> std::io::Result<()> {
        let mut out_bytes = Vec::with_capacity(4096);
        use std::io::Write;
        out_bytes.write_all(&[self.name_len])?;
        out_bytes.write_all(&self.name[..(self.name_len as usize)])?;
        self.data.write(&mut out_bytes)?;
        target.write_all(&(self.data.section_type() as u16).to_le_bytes())?;
        target.write_all(&(out_bytes.len() as u64).to_le_bytes())?;
        target.write_all(&[crate::tmf::CompressionType::None as u8])?;

        target.write_all(&out_bytes)
    }
    //return Err(std::io::Error::new(std::io::ErrorKind::Other,format!("Invalid custom se"),)),
    pub(crate) fn read<R: std::io::Read>(mut src: R, kind: SectionType) -> std::io::Result<Self> {
        let mut name_len = [0];
        src.read_exact(&mut name_len)?;
        let name_len = name_len[0];
        let mut name = [0; u8::MAX as usize];
        src.read_exact(&mut name[..(name_len as usize)])?;
        match kind {
            SectionType::CustomIndexSegment => {
                let result = crate::vertices::read_triangles(&mut src)?;
                Ok(Self::new_raw(
                    CustomData::new_index(&result, None),
                    name,
                    name_len,
                ))
            }
            _ => panic!("InternalError: Invalid custom section type, must be custom!"),
        }
    }
}
#[cfg(test)]
fn init_test_env() {
    std::fs::create_dir_all("target/test_res").unwrap();
}
#[test]
#[cfg(all(feature = "obj_import", test))]
fn index_data() {
    use crate::{TMFMesh, TMFPrecisionInfo};
    init_test_env();
    let mut file = std::fs::File::open("testing/susan.obj").unwrap();
    let (mut tmf_mesh, name) = TMFMesh::read_from_obj_one(&mut file).unwrap();
    let index_data: [IndexType; 10] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    let index_data_seg =
        CustomDataSegment::new(CustomData::from(&index_data[..]), "custom_index").unwrap();
    tmf_mesh.add_custom_data(index_data_seg);
    tmf_mesh.verify().unwrap();
    assert!(name == "Suzanne", "Name should be Suzanne but is {name}");
    let prec = TMFPrecisionInfo::default();
    let mut out = Vec::new();
    {
        tmf_mesh.write_tmf_one(&mut out, &prec, name).unwrap();
    }
    let (r_mesh, name) = TMFMesh::read_tmf_one(&mut (&out as &[u8])).unwrap();
    assert!(name == "Suzanne", "Name should be Suzanne but is {name}");
    r_mesh.verify().unwrap();
    let read_indices = tmf_mesh
        .lookup_custom_data("custom_index")
        .expect("Could not find the custom index array!");
    let (read_indices, _) = read_indices.is_index().unwrap();
    assert_eq!(index_data, read_indices);
}
