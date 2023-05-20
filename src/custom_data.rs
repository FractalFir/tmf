use crate::IndexType;
use crate::tmf::SectionType;
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
        Ok(Self {
            name,
            name_len,
            data,
        })
    }
}
#[derive(Clone)]
pub enum CustomData {
    CustomIndex(Box<[IndexType]>),
}
impl CustomData{
    fn section_type(&self)->SectionType{
        match self{
            Self::CustomIndex(_)=>SectionType::CustomIndexSegment,
        }
    }
}
impl From<&[IndexType]> for CustomData {
    fn from(indices: &[IndexType]) -> Self {
        Self::CustomIndex(indices.into())
    }
}
impl CustomDataSegment{
   pub(crate) fn write<W:std::io::Write>(&self,target:&mut W)->std::io::Result<()>{
        target.write(&(self.data.section_type() as u16).to_le_bytes())?;
        todo!();
    }
    fn read<R:std::io::Read>(src:&mut R)->Self{
        todo!();
    }
}
#[cfg(test)]
fn init_test_env() {
    std::fs::create_dir_all("target/test_res").unwrap();
}
#[test]
#[cfg(all(feature = "obj_import",test))]
fn index_data() {
    use crate::{TMFMesh,TMFPrecisionInfo};
    init_test_env();
    let mut file = std::fs::File::open("testing/susan.obj").unwrap();
    let (mut tmf_mesh, name) = TMFMesh::read_from_obj_one(&mut file).unwrap();
    let index_data:[IndexType;10] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    let index_data_seg = CustomDataSegment::new(CustomData::from(&index_data[..]), "custom_index").unwrap();
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
}
