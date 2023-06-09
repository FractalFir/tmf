#![allow(dead_code)]
use crate::unaligned_rw::{UnalignedRWMode, UnalignedReader, UnalignedWriter};
use crate::IndexType;
fn calc_sw_size(prec: u8) -> usize {
    1_usize << prec
}
fn find_longest_match(data: &[u64], pos: usize, scrollback_limit: usize) -> (usize, usize) {
    let mut best_offset = 0;
    let mut best_len = 0;
    let start = if pos < scrollback_limit {
        0
    } else {
        pos - scrollback_limit
    };
    for offset in start..pos {
        let len = match_len(data, offset, pos);
        if len > best_len {
            best_offset = pos - offset;
            best_len = len;
        }
    }
    (best_offset, best_len)
}
fn match_len(data: &[u64], offset: usize, end: usize) -> usize {
    let mut offset = offset;
    let mut pos = end;
    let mut len = 0;
    while offset < pos && pos < data.len() && data[offset] == data[pos] {
        offset += 1;
        pos += 1;
        len += 1;
    }
    len
}
use crate::vertices::calc_prec;
fn save_compressed_array<T, Mapper: Fn(&T) -> u64, W: std::io::Write>(
    data: &[T],
    mapper: Mapper,
    prec: UnalignedRWMode,
    target: &mut W,
) -> std::io::Result<()> {
    println!("data.len():{}", data.len());
    let data: Box<_> = data.iter().map(mapper).collect();
    let mut writer = UnalignedWriter::new(target);
    let mut pos = 0;
    let scrollback_limit = 127;
    println!("data.len():{}", data.len());
    while pos < data.len() {
        let (offset, len) = find_longest_match(&data, pos, scrollback_limit);
        //TODO:Write
        let write_raw = offset == 0 || len < 2;
        writer.write_bit(write_raw)?;
        if write_raw {
            println!("({})", data[pos]);
            writer.write_unaligned(prec, data[pos])?;
            pos += 1;
        } else {
            writer.write_unaligned(prec, len as u64)?;
            writer.write_unaligned(prec, offset as u64)?;
            pos += len;
            println!("({offset}, {len})");
        }
    }
    writer.flush()
}
fn read_compressed_array<T, Mapper: Fn(u64) -> T, R: std::io::Read>(
    mapper: Mapper,
    prec: UnalignedRWMode,
    length: usize,
    src: &mut R,
) -> std::io::Result<Box<[T]>> {
    let mut output = Vec::with_capacity(length);
    let mut reader = UnalignedReader::new(src);
    while output.len() < length {
        //println!("POS: {pos}");
        let is_raw = reader.read_bit()?;
        let data_or_len = reader.read_unaligned(prec)?;
        if is_raw {
            output.push(data_or_len);
            print!("{data_or_len},");
        } else {
            // Impossible
            #[allow(clippy::cast_possible_truncation)]
            let offset = reader.read_unaligned(prec)? as usize;
            let Ok(len) = usize::try_from(data_or_len) else { return Err(std::io::Error::new(std::io::ErrorKind::Other, "Data segment length bigger than size of this devices address space.")) };
            let start = output.len() - offset;
            for i in start..(start + len) {
                let current = output[i];
                print!("{current},");
                output.push(current);
            }
        }
    }
    let output: Box<_> = output.into_iter().map(mapper).collect();
    Ok(output)
}
pub(crate) fn save_triangles_lzz<W: std::io::Write>(
    triangles: &[IndexType],
    max_index: usize,
    writer: &mut W,
) -> std::io::Result<()> {
    let min = *triangles.iter().min().unwrap_or(&0);
    let precision = calc_prec(max_index);
    writer.write_all(&precision.to_le_bytes())?;
    writer.write_all(&(triangles.len() as u64).to_le_bytes())?;
    writer.write_all(&u64::from(min).to_le_bytes())?;
    let precision = UnalignedRWMode::precision_bits(precision);
    save_compressed_array(triangles, |index| u64::from(index - min), precision, writer)?;
    //writer.flush()
    Ok(())
}
#[test]
fn encode_ascii_lzz() {
    let data = b"LZ77 algorithms achieve compression by replacing repeated occurrences of data with references to a single copy of that data existing earlier in the uncompressed data stream. A match is encoded by a pair of numbers called a length-distance pair, which is equivalent to the statement \"each of the next length characters is equal to the characters exactly distance characters behind it in the uncompressed stream\". (The distance is sometimes called the offset instead.) To spot matches, the encoder must keep track of some amount of the most recent data, such as the last 2 KB, 4 KB, or 32 KB. The structure in which this data is held is called a sliding window, which is why LZ77 is sometimes called sliding-window compression. The encoder needs to keep this data to look for matches, and the decoder needs to keep this data to interpret the matches the encoder refers to. The larger the sliding window is, the longer back the encoder may search for creating references";
    let mut out = Vec::new();
    save_compressed_array(
        &data[..],
        |input: &u8| *input as u64,
        UnalignedRWMode::precision_bits(7),
        &mut out,
    )
    .unwrap();
    let _data = read_compressed_array(
        |input: u64| input as u8,
        UnalignedRWMode::precision_bits(7),
        data.len(),
        &mut &out[..],
    )
    .unwrap();
    //panic!("Compressed is:{}%",(out.len() as f64 / data.len() as f64)*100.0);
}
#[test]
fn compress_triangles() {}
