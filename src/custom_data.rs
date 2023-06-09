use crate::read_extension::ReadExt;
use crate::tmf::SectionType;
use crate::{FloatType, IndexType, TMFImportError, Vector4, MAX_SEG_SIZE};
#[derive(Clone, Debug)]
pub(crate) struct CustomDataSegment {
    name: Vec<u8>,
    name_len: u8,
    data: CustomData,
}
#[derive(Debug, Copy, Clone)]
/// An enum representing error, which occurred while adding a segment counting custom mesh data.
pub enum DataSegmentError {
    /// A segment must have a name, but it's name was set to an empty string.
    NoName,
    /// Name over 255 byte limit.
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
    CustomIntiger(Box<[IndexType]>, usize),
    CustomFloat(Box<[FloatType]>, FloatType),
    CustomColorRGBA(Box<[Vector4]>, FloatType),
}
impl CustomData {
    /// Returns the index data if custom segment is an index segment. Returns the index array and max index.
    pub fn as_index(&self) -> Option<(&[IndexType], usize)> {
        match self {
            Self::CustomIndex(array, max_index) => Some((array, *max_index)),
            _ => None,
        }
    }
    pub fn as_intiger(&self) -> Option<(&[IndexType], usize)> {
        match self {
            Self::CustomIntiger(array, max_index) => Some((array, *max_index)),
            _ => None,
        }
    }
    pub fn as_float(&self) -> Option<(&[FloatType], FloatType)> {
        match self {
            Self::CustomFloat(array, prec) => Some((array, *prec)),
            _ => None,
        }
    }
    pub fn as_color_rgba(&self) -> Option<(&[Vector4], FloatType)> {
        match self {
            Self::CustomColorRGBA(array, prec) => Some((array, *prec)),
            _ => None,
        }
    }
    fn write<W: std::io::Write>(&self, target: &mut W) -> std::io::Result<()> {
        match self {
            Self::CustomIndex(data, max_index) => {
                crate::vertices::save_triangles(data, *max_index, target)
            }
            Self::CustomIntiger(data, max_index) => {
                crate::vertices::save_triangles(data, *max_index, target)
            }
            Self::CustomFloat(data, prec) => {
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
            Self::CustomColorRGBA(data, prec) => {
                use crate::unaligned_rw::{UnalignedRWMode, UnalignedWriter};
                let prec_bits = (1.0 / prec.min(1.0)).log2().ceil() as u8;
                let mul = ((1 << prec_bits) - 1) as FloatType;
                target.write_all(&(data.len() as u64).to_le_bytes())?;
                target.write_all(&[prec_bits])?;
                let prec = UnalignedRWMode::precision_bits(prec_bits);
                let mut writer = UnalignedWriter::new(target);
                for entry in data.iter() {
                    let entry_r = (entry.0 * mul) as u64;
                    let entry_g = (entry.1 * mul) as u64;
                    let entry_b = (entry.2 * mul) as u64;
                    let entry_a = (entry.3 * mul) as u64;
                    writer.write_unaligned(prec, entry_r)?;
                    writer.write_unaligned(prec, entry_g)?;
                    writer.write_unaligned(prec, entry_b)?;
                    writer.write_unaligned(prec, entry_a)?;
                }
                Ok(())
            }
        }?;
        Ok(())
    }
    fn section_type(&self) -> SectionType {
        match self {
            Self::CustomIndex(_, _) => SectionType::CustomIndexSegment,
            Self::CustomIntiger(_, _) => SectionType::CustomIntigerSegment,
            Self::CustomFloat(_, _) => SectionType::CustomFloatSegment,
            Self::CustomColorRGBA(_, _) => SectionType::CustomColorRGBASegment,
        }
    }
    fn new_float(floats: &[FloatType], prec: FloatType) -> Self {
        Self::CustomFloat(floats.into(), prec)
    }
    fn new_color_rgba(colors: &[Vector4], prec: FloatType) -> Self {
        Self::CustomColorRGBA(colors.into(), prec)
    }
    fn new_index(indices: &[IndexType], max_index: Option<usize>) -> Self {
        let max_index = match max_index {
            Some(max_index) => max_index,
            None => indices.iter().max().copied().unwrap_or(0) as usize,
        };
        Self::CustomIndex(indices.into(), max_index)
    }
    fn new_intiger(indices: &[IndexType], max_intiger: Option<usize>) -> Self {
        let max_intiger = match max_intiger {
            Some(max_intiger) => max_intiger,
            None => indices.iter().max().copied().unwrap_or(0) as usize,
        };
        Self::CustomIntiger(indices.into(), max_intiger)
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
impl From<&[Vector4]> for CustomData {
    fn from(colors: &[Vector4]) -> Self {
        Self::new_color_rgba(colors, 0.01)
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
        let name_len = src.read_u8()?;
        let mut name = [0; u8::MAX as usize];
        src.read_exact(&mut name[..(name_len as usize)])?;
        match kind {
            /*SectionType::CustomColorSegment =>{

            }*/
            SectionType::CustomIndexSegment => {
                let result = crate::vertices::read_triangles(&mut src, ctx)?;
                Ok(Self::new_raw(
                    CustomData::new_index(&result, None),
                    name,
                    name_len,
                ))
            }
            SectionType::CustomIntigerSegment => {
                let result = crate::vertices::read_triangles(&mut src, ctx)?;
                Ok(Self::new_raw(
                    CustomData::new_intiger(&result, None),
                    name,
                    name_len,
                ))
            }
            SectionType::CustomFloatSegment => {
                use crate::unaligned_rw::{UnalignedRWMode, UnalignedReader};
                let len = src.read_u64()?;
                if len > MAX_SEG_SIZE as u64 {
                    return Err(TMFImportError::SegmentTooLong);
                }
                let min = src.read_f64()?;
                let max = src.read_f64()?;
                let prec = src.read_u8()?;
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
            SectionType::CustomColorRGBASegment => {
                use crate::unaligned_rw::{UnalignedRWMode, UnalignedReader};
                let len = src.read_u64()?;
                let prec_bits = src.read_u8()?;
                let prec = UnalignedRWMode::precision_bits(prec_bits);
                let mut reader = UnalignedReader::new(src);
                let mut res = vec![(0.0, 0.0, 0.0, 0.0); len as usize];
                let div = ((1_u64 << prec_bits) - 1) as f64;
                for vec4 in &mut res {
                    let (r, g) = reader.read2_unaligned(prec)?;
                    let (b, a) = reader.read2_unaligned(prec)?;
                    *vec4 = (
                        ((r as f64) / div) as FloatType,
                        ((g as f64) / div) as FloatType,
                        ((b as f64) / div) as FloatType,
                        ((a as f64) / div) as FloatType,
                    );
                }
                let prec = ((1.0 / ((1_u64 << prec_bits) as f64)) as FloatType) * 0.99999;
                Ok(Self::new_raw(
                    CustomData::new_color_rgba(&res, prec),
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
    let (read_indices, _) = read_indices.as_index().unwrap();
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

    let (read_floats, _) = read_floats.as_float().unwrap();

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
#[test]
#[cfg(all(feature = "obj_import", test))]
fn color_rgba_data() {
    use crate::{TMFMesh, TMFPrecisionInfo};
    init_test_env();
    let mut file = std::fs::File::open("testing/susan.obj").unwrap();
    let (mut tmf_mesh, name) = TMFMesh::read_from_obj_one(&mut file).unwrap();
    let color_rgba_data: [Vector4; 3] = [
        (0.9, 0.19, 0.2, 0.7867),
        (0.431224, 0.534345, 0.64336, 0.78634),
        (0.776565, 0.87575, 0.954, 0.3543),
    ];
    tmf_mesh
        .add_custom_data(color_rgba_data[..].into(), "custom_color_rgba")
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
    let read_color_rgbas = r_mesh
        .lookup_custom_data("custom_color_rgba")
        .expect("Could not find the custom color_rgba array!");

    let (read_color_rgbas, _) = read_color_rgbas.as_color_rgba().unwrap();
    for index in 0..read_color_rgbas.len() {
        assert!(
            (read_color_rgbas[index].0 - color_rgba_data[index].0).abs() <= 0.01,
            "{} diff {:?} {:?} > 0.01!",
            index,
            read_color_rgbas[index],
            color_rgba_data[index]
        );
        assert!(
            (read_color_rgbas[index].1 - color_rgba_data[index].1).abs() <= 0.01,
            "{} diff {:?} {:?} > 0.01!",
            index,
            read_color_rgbas[index],
            color_rgba_data[index]
        );
        assert!(
            (read_color_rgbas[index].2 - color_rgba_data[index].2).abs() <= 0.01,
            "{} diff {:?} {:?} > 0.01!",
            index,
            read_color_rgbas[index],
            color_rgba_data[index]
        );
        assert!(
            (read_color_rgbas[index].3 - color_rgba_data[index].3).abs() <= 0.01,
            "{} diff {:?} {:?} > 0.01!",
            index,
            read_color_rgbas[index],
            color_rgba_data[index]
        );
    }
}
