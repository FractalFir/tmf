use crate::unaligned_rw::{UnalignedRWMode, UnalignedReader, UnalignedWriter};
use crate::TMFImportError;
use crate::MAX_SEG_SIZE;
use crate::{FloatType, IndexType, Vector3};
use std::io::{Read, Write};
#[derive(Clone, Copy, PartialEq)]
/// Setting dictating how much the length of any edge can change because of vertex precision loss during saving. This is expressed as a fraction of the length of the shortest edge.
///```
/// # use tmf::VertexPrecisionMode;
/// // No edge will deviate by more than 0.1(10%) of the shortest edge in the model,
/// // eg. for edges 10, 234, and 1, their length will not change by more than  1*0.1 = 0.1
/// let ten_percent = VertexPrecisionMode(0.1);
/// // No edge will deviate by more than 0.025(2.5%) of the shortest edge in the model,
/// // eg. for edges 10, 234, and 1, their length will not change by more than  1*0.025 = 0.025
/// let two_and_half_percent = VertexPrecisionMode(0.025);
///```
pub struct VertexPrecisionMode(pub FloatType);
impl Default for VertexPrecisionMode {
    /// Default maximum edge deviation is 0.1 or 10%
    ///```
    ///# use tmf::VertexPrecisionMode;
    /// let mode = VertexPrecisionMode(0.1);
    /// let default_mode =  VertexPrecisionMode::default();
    /// // The same
    /// assert!(mode == default_mode);
    fn default() -> Self {
        Self(0.1)
    }
}
pub fn save_tmf_vertices<W: Write>(
    vertices: &[Vector3],
    precision: VertexPrecisionMode,
    writer: &mut W,
    shortest_edge: FloatType,
) -> std::io::Result<()> {
    let mut min_x = vertices[0].0;
    let mut max_x = vertices[0].0;
    let mut min_y = vertices[0].1;
    let mut max_y = vertices[0].1;
    let mut min_z = vertices[0].2;
    let mut max_z = vertices[0].2;
    writer.write_all(&(vertices.len() as u64).to_le_bytes())?;
    //Get min and max x,y,z coords
    for vertex in vertices {
        min_x = min_x.min(vertex.0);
        max_x = max_y.max(vertex.1);
        min_y = min_z.min(vertex.2);
        max_y = max_x.max(vertex.0);
        min_z = min_y.min(vertex.1);
        max_z = max_z.max(vertex.2);
    }
    writer.write_all(&(min_x as f64).to_le_bytes())?;
    writer.write_all(&(max_x as f64).to_le_bytes())?;
    writer.write_all(&(min_y as f64).to_le_bytes())?;
    writer.write_all(&(max_y as f64).to_le_bytes())?;
    writer.write_all(&(min_z as f64).to_le_bytes())?;
    writer.write_all(&(max_z as f64).to_le_bytes())?;
    //Calculate size of the model
    let sx = max_x - min_x;
    let sy = max_y - min_y;
    let sz = max_z - min_z;
    // Ensure model size is not 0(prevents bugs)
    let sx = sx.max(0.000_001);
    let sy = sy.max(0.000_001);
    let sz = sz.max(0.000_001);
    //Calculate minimal increment
    let inc_x = (shortest_edge / sx) * precision.0;
    let inc_y = (shortest_edge / sy) * precision.0;
    let inc_z = (shortest_edge / sy) * precision.0;
    //Calculate write precision
    let prec_x = (1.0 / inc_x).log2().ceil() as u8;
    let prec_y = (1.0 / inc_y).log2().ceil() as u8;
    let prec_z = (1.0 / inc_z).log2().ceil() as u8;
    //Ensure precision is never below 1
    let prec_x = prec_x.max(1);
    let prec_y = prec_y.max(1);
    let prec_z = prec_z.max(1);
    // Write precision info
    writer.write_all(&[prec_x])?;
    writer.write_all(&[prec_y])?;
    writer.write_all(&[prec_z])?;
    // Calculate float save multiplier
    let mul_x = ((1 << prec_x) - 1) as FloatType;
    let mul_y = ((1 << prec_y) - 1) as FloatType;
    let mul_z = ((1 << prec_z) - 1) as FloatType;
    // Create unaligned rw modes
    let prec_x = UnalignedRWMode::precision_bits(prec_x);
    let prec_y = UnalignedRWMode::precision_bits(prec_y);
    let prec_z = UnalignedRWMode::precision_bits(prec_z);
    // Create writer
    let mut writer = UnalignedWriter::new(writer);
    for vertex in vertices {
        let x = (((vertex.0 - min_x) / sx) * mul_x) as u64;
        let y = (((vertex.1 - min_y) / sy) * mul_y) as u64;
        let z = (((vertex.2 - min_z) / sz) * mul_z) as u64;

        writer.write_unaligned(prec_x, x)?;
        writer.write_unaligned(prec_y, y)?;
        writer.write_unaligned(prec_z, z)?;
    }
    writer.flush()?;
    Ok(())
}
fn read_f64<R: Read>(reader: &mut R) -> std::io::Result<f64> {
    let mut tmp = [0; std::mem::size_of::<f64>()];
    reader.read_exact(&mut tmp)?;
    Ok(f64::from_le_bytes(tmp))
}
pub fn read_tmf_vertices<R: Read>(reader: &mut R) -> Result<Box<[Vector3]>, TMFImportError> {
    let vertex_count = {
        let mut tmp = [0; std::mem::size_of::<u64>()];
        reader.read_exact(&mut tmp)?;
        u64::from_le_bytes(tmp)
    } as usize;
    // Read data bounding box
    let min_x = read_f64(reader)? as FloatType;
    let max_x = read_f64(reader)? as FloatType;
    let min_y = read_f64(reader)? as FloatType;
    let max_y = read_f64(reader)? as FloatType;
    let min_z = read_f64(reader)? as FloatType;
    let max_z = read_f64(reader)? as FloatType;
    // Read precision
    let prec_x = {
        let mut tmp = [0];
        reader.read_exact(&mut tmp)?;
        tmp[0]
    };
    if prec_x >= u64::BITS as u8 {
        return Err(TMFImportError::InvalidPrecision(prec_x));
    }
    let prec_y = {
        let mut tmp = [0];
        reader.read_exact(&mut tmp)?;
        tmp[0]
    };
    if prec_y >= u64::BITS as u8 {
        return Err(TMFImportError::InvalidPrecision(prec_y));
    }
    let prec_z = {
        let mut tmp = [0];
        reader.read_exact(&mut tmp)?;
        tmp[0]
    };
    if prec_z >= u64::BITS as u8 {
        return Err(TMFImportError::InvalidPrecision(prec_z));
    }
    // Calculate float read divisor
    let div_x = ((1_u64 << prec_x) - 1) as FloatType;
    let div_y = ((1_u64 << prec_y) - 1) as FloatType;
    let div_z = ((1_u64 << prec_z) - 1) as FloatType;
    // Create unaligned rw modes
    let prec_x = UnalignedRWMode::precision_bits(prec_x);
    let prec_y = UnalignedRWMode::precision_bits(prec_y);
    let prec_z = UnalignedRWMode::precision_bits(prec_z);
    //Calculate size of the model
    let sx = max_x - min_x;
    let sy = max_y - min_y;
    let sz = max_z - min_z;
    //Create arrays to write data into
    if vertex_count > MAX_SEG_SIZE {
        return Err(TMFImportError::SegmentTooLong);
    }
    let mut vertices = Vec::with_capacity(vertex_count);
    let mut reader = UnalignedReader::new(reader);
    for _ in 0..vertex_count {
        //let (x,y) = reader.read_pair_unaligned(prec_x,prec_y)?;
        let x = reader.read_unaligned(prec_x)?;
        let y = reader.read_unaligned(prec_y)?;
        let z = reader.read_unaligned(prec_z)?;
        let x = (x as FloatType) / div_x;
        let y = (y as FloatType) / div_y;
        let z = (z as FloatType) / div_z;
        let x = x * sx + min_x;
        let y = y * sy + min_y;
        let z = z * sz + min_z;
        vertices.push((x, y, z));
    }
    Ok(vertices.into())
}
pub(crate) fn calc_prec(max: usize) -> u8 {
    (max as FloatType + 1.0).log2().ceil() as u8
}
pub fn save_triangles<W: Write>(
    triangles: &[IndexType],
    max_index: usize,
    writer: &mut W,
) -> std::io::Result<()> {
    let min = *triangles.iter().min().unwrap_or(&0);
    let triangles: Vec<IndexType> = triangles.iter().map(|i| i - min).collect();
    let precision = calc_prec(max_index);
    writer.write_all(&precision.to_le_bytes())?;
    writer.write_all(&(triangles.len() as u64).to_le_bytes())?;
    writer.write_all(&(min as u64).to_le_bytes())?;
    let precision = UnalignedRWMode::precision_bits(precision);
    let mut writer = UnalignedWriter::new(writer);
    for index in triangles {
        writer.write_unaligned(precision, index as u64)?;
    }
    writer.flush()
}
pub(crate) fn read_triangles<R: Read>(
    reader: &mut R,
    ctx: &crate::tmf_importer::TMFImportContext,
) -> Result<Box<[IndexType]>, TMFImportError> {
    let precision = {
        let mut tmp = [0];
        reader.read_exact(&mut tmp)?;
        tmp[0]
    };
    let length = {
        let mut tmp = [0; std::mem::size_of::<u64>()];
        reader.read_exact(&mut tmp)?;
        u64::from_le_bytes(tmp)
    };
    let min = ctx.read_traingle_min(reader)?;
    if length > MAX_SEG_SIZE as u64 {
        return Err(TMFImportError::SegmentTooLong);
    }
    let mut res = Vec::with_capacity(length as usize);
    let precision = UnalignedRWMode::precision_bits(precision);
    let mut reader = UnalignedReader::new(reader);
    for _ in 0..(length/2) {
        let (i1,i2) = reader.read2_unaligned(precision)?;
        res.push((i1 + min) as IndexType);
        res.push((i2 + min) as IndexType);
    }
    for i in 0..length%2{
         res.push((reader.read_unaligned(precision)? + min) as IndexType);
    }
    Ok(res.into())
}
#[cfg(test)]
mod testing {
    fn dst(a: Vector3, b: Vector3) -> FloatType {
        let dx = a.0 - b.0;
        let dy = a.1 - b.1;
        let dz = a.2 - b.2;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
    use super::*;
    /*
    #[test]
    fn rw_triangles() {
        use rand::{thread_rng, Rng};
        let mut rng = thread_rng();
        let vertex_count = (rng.gen::<IndexType>() % 0x800) + 0x800;
        let triangle_count = (rng.gen::<IndexType>() % 0x800) + 0x800;
        let mut triangles = Vec::with_capacity(triangle_count as usize);
        for _ in 0..triangle_count {
            triangles.push(rng.gen::<IndexType>() % vertex_count);
        }
        let mut res = Vec::with_capacity(vertex_count as usize);
        {
            save_triangles(&triangles, triangles.len(), &mut res).unwrap();
        }
        let r_triangles = read_triangles(&mut (&res as &[u8])).unwrap();
        assert!(triangles.len() == r_triangles.len());
        for i in 0..(triangle_count as usize) {
            assert!(r_triangles[i] == triangles[i]);
        }
    }*/
    #[test]
    fn rw_vertices() {
        use rand::{thread_rng, Rng};
        let mut rng = thread_rng();
        let vertex_count = (rng.gen::<IndexType>() % 0x800) + 0x800;
        let mut vertices = Vec::with_capacity(vertex_count as usize);
        for _ in 0..vertex_count {
            let x = (rng.gen::<FloatType>() - 0.5) * 2.0;
            let y = (rng.gen::<FloatType>() - 0.5) * 2.0;
            let z = (rng.gen::<FloatType>() - 0.5) * 2.0;
            vertices.push((x, y, z));
        }
        let mut res = Vec::with_capacity(vertex_count as usize);
        {
            save_tmf_vertices(&vertices, VertexPrecisionMode(0.01), &mut res, 0.1).unwrap();
        }
        let r_vertices = read_tmf_vertices(&mut (&res as &[u8])).unwrap();
        assert!(r_vertices.len() == vertices.len());
        for i in 0..vertices.len() {
            assert!(
                dst(r_vertices[i], vertices[i]) < 0.01,
                "{:?} {:?} {}",
                r_vertices[i],
                vertices[i],
                dst(r_vertices[i], vertices[i])
            );
        }
    }
}
