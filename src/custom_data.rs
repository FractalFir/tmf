use crate::tmf::SectionType;
use crate::{FloatType, IndexType, TMFImportError, MAX_SEG_SIZE};
#[derive(Clone, Debug)]
pub(crate) struct CustomDataSegment {
    name: Vec<u8>,
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
        let raw = raw[0..(name_len as usize)].into();
        Self {
            name: raw,
            name_len,
            data,
        }
    }
    pub(crate) fn custom_data(&self) -> &CustomData {
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
        name[..bytes.len()].copy_from_slice(bytes);
        Ok(Self::new_raw(data, name, name_len))
    }
}
/// Custom mesh data.
#[derive(Clone, Debug)]
pub enum CustomData {
    CustomIndex(Box<[IndexType]>, usize),
    CustomFloat(Box<[FloatType]>, FloatType),
}
impl CustomData {
    /// Returns the index data if custom segment is an index segment. Returns the index array and max index.
    pub fn is_index(&self) -> Option<(&[IndexType], usize)> {
        match self {
            Self::CustomIndex(array, max_index) => Some((array, *max_index)),
            _ => None,
        }
    }
    pub fn is_float(&self) -> Option<(&[FloatType], FloatType)> {
        match self {
            Self::CustomFloat(array, prec) => Some((array, *prec)),
            _ => None,
        }
    }
    fn write<W: std::io::Write>(&self, target: &mut W) -> std::io::Result<()> {
        match self {
            Self::CustomIndex(data, max_index) => {
                crate::vertices::save_triangles(data, *max_index, target)
            }
            CustomData::CustomFloat(data, prec) => {
                use crate::unaligned_rw::{UnalignedRWMode, UnalignedWriter};
                let mut max = FloatType::MIN;
                let mut min = FloatType::MAX;
                for entry in data.iter() {
                    max = max.max(*entry);
                    min = min.min(*entry);
                }
                let span = max - min;
                let prec = (span / prec).log2().ceil() as u8;
                //Ensure precision is never 0(messes up the reader/writer);
                let prec = prec.max(1);
                let mul = ((1 << prec) - 1) as FloatType;
                target.write_all(&(data.len() as u64).to_le_bytes())?;
                target.write_all(&(min as f64).to_le_bytes())?;
                target.write_all(&(max as f64).to_le_bytes())?;
                target.write_all(&[prec])?;
                let prec = UnalignedRWMode::precision_bits(prec);
                let mut writer = UnalignedWriter::new(target);
                for entry in data.iter() {
                    let entry = (((entry - min) / span) * mul) as u64;
                    writer.write_unaligned(prec, entry)?;
                }
                Ok(())
            }
        }?;
        Ok(())
    }
    fn section_type(&self) -> SectionType {
        match self {
            Self::CustomIndex(_, _) => SectionType::CustomIndexSegment,
            Self::CustomFloat(_, _) => SectionType::CustomFloatSegment,
        }
    }
    fn new_float(floats: &[FloatType], prec: FloatType) -> Self {
        Self::CustomFloat(floats.into(), prec)
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
        Self::new_index(indices, None)
    }
}
impl From<&[FloatType]> for CustomData {
    fn from(floats: &[FloatType]) -> Self {
        Self::new_float(floats, 0.01)
    }
}
impl CustomDataSegment {
    pub(crate) fn encode<W: std::io::Write>(&self, target: &mut W) -> std::io::Result<SectionType> {
        target.write_all(&[self.name_len])?;
        target.write_all(&self.name[..(self.name_len as usize)])?;
        self.data.write(target)?;
        Ok(self.data.section_type())
    }
    //return Err(std::io::Error::new(std::io::ErrorKind::Other,format!("Invalid custom se"),)),
    pub(crate) fn read<R: std::io::Read>(
        mut src: R,
        kind: SectionType,
        ctx: &crate::tmf_importer::TMFImportContext,
    ) -> Result<Self, TMFImportError> {
        let mut name_len = [0];
        src.read_exact(&mut name_len)?;
        let name_len = name_len[0];
        let mut name = [0; u8::MAX as usize];
        src.read_exact(&mut name[..(name_len as usize)])?;
        match kind {
            SectionType::CustomIndexSegment => {
                let result = crate::vertices::read_triangles(&mut src, ctx)?;
                Ok(Self::new_raw(
                    CustomData::new_index(&result, None),
                    name,
                    name_len,
                ))
            }
            SectionType::CustomFloatSegment => {
                use crate::unaligned_rw::{UnalignedRWMode, UnalignedReader};
                let len = {
                    let mut tmp = [0; std::mem::size_of::<u64>()];
                    src.read_exact(&mut tmp)?;
                    u64::from_le_bytes(tmp)
                };
                if len > MAX_SEG_SIZE as u64 {
                    return Err(TMFImportError::SegmentTooLong);
                }
                let min = {
                    let mut tmp = [0; std::mem::size_of::<f64>()];
                    src.read_exact(&mut tmp)?;
                    f64::from_le_bytes(tmp)
                };
                let max = {
                    let mut tmp = [0; std::mem::size_of::<f64>()];
                    src.read_exact(&mut tmp)?;
                    f64::from_le_bytes(tmp)
                };
                let mut prec = [0];
                src.read_exact(&mut prec)?;
                let prec = prec[0];
                if prec >= u64::BITS as u8 {
                    return Err(TMFImportError::InvalidPrecision(prec));
                }
                let prec_bits = prec;
                let div = ((1_u64 << prec_bits) - 1) as f64;
                let span = max - min;
                let prec = UnalignedRWMode::precision_bits(prec);
                let mut reader = UnalignedReader::new(src);
                let mut res = vec![0.0; len as usize];
                for float in &mut res {
                    let val = reader.read_unaligned(prec)?;
                    *float = (((val as f64) / div) * span + min) as FloatType;
                }
                let prec = (((max - min) / ((1_u64 << prec_bits) as f64)) as FloatType) * 0.99999;
                Ok(Self::new_raw(
                    CustomData::new_float(&res, prec),
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
    tmf_mesh
        .add_custom_data(index_data[..].into(), "custom_index")
        .unwrap();
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
    let read_indices = r_mesh
        .lookup_custom_data("custom_index")
        .expect("Could not find the custom index array!");
    let (read_indices, _) = read_indices.is_index().unwrap();
    assert_eq!(index_data, read_indices);
}
#[test]
#[cfg(all(feature = "obj_import", test))]
fn float_data() {
    use crate::{TMFMesh, TMFPrecisionInfo};
    init_test_env();
    let mut file = std::fs::File::open("testing/susan.obj").unwrap();
    let (mut tmf_mesh, name) = TMFMesh::read_from_obj_one(&mut file).unwrap();
    let float_data: [FloatType; 10] = [
        -7.0, 1.9, -2.0, 3.7867, 4.31224, 5.34345, 6.4336, 7.76565, 8.7575, 9.54,
    ];
    tmf_mesh
        .add_custom_data(float_data[..].into(), "custom_float")
        .unwrap();
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
    let read_floats = r_mesh
        .lookup_custom_data("custom_float")
        .expect("Could not find the custom float array!");

    let (read_floats, _) = read_floats.is_float().unwrap();

    for index in 0..read_floats.len() {
        assert!(
            (read_floats[index] - float_data[index]).abs() <= 0.01,
            "{} diff {} {} > 0.01!",
            index,
            read_floats[index],
            float_data[index]
        );
    }
}
