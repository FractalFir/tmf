use crate::model_importer::ModelImporter;
use crate::{FloatType, IndexType, TMFMesh, Vector2, Vector3};
use std::io::{BufReader, BufWriter, Error, ErrorKind, Read, Result, Write};
const SMALL_VEC_CAP:usize = 8;
fn parse_line(line: Result<String>, oi: &mut ModelImporter) -> Result<Option<(TMFMesh, String)>> {
    let line = line?;
    // Split the line by white spaces and '/' sign used in triangles
    let mut split = line.split(&[' ', '/']);
    // Get the beginning of the line
    let beg = match_split(split.next())?;
    match beg {
        "#" => (), //Ignoring comments
        "mtllib" => (),
        "usemtl" => (),
        "s" => (), //Ignore smoothness info
        "v" => oi.push_vertex(load_vec3(&mut split)?),
        "vn" => oi.push_normal(load_vec3(&mut split)?),
        "vt" => oi.push_uv(load_vec2(&mut split)?),
        "f" => load_face(&mut split, oi)?,
        "o" => {
            let name = match_split(split.next())?.to_owned();
            return Ok(oi.next_mesh(name));
        }
        _ => todo!(),
    }
    Ok(None)
}
pub fn read_from_obj<R: Read>(reader: &mut R) -> Result<Vec<(TMFMesh, String)>> {
    let reader = BufReader::new(reader);
    use std::io::BufRead;
    let mut oi = ModelImporter::new();
    let mut lines = reader.lines();
    let mut res = Vec::new();
    for line in lines {
        match parse_line(line, &mut oi)? {
            Some(mesh_and_name) => res.push(mesh_and_name),
            None => (),
        }
    }
    res.push(oi.finish());
    Ok(res)
}
fn parse_float_type(float: &str) -> Result<FloatType> {
    match float.parse::<FloatType>() {
        Ok(float) => Ok(float),
        Err(err) => Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            err.to_string(),
        )),
    }
}
fn parse_index(uint: &str) -> Result<IndexType> {
    match uint.parse::<IndexType>() {
        Ok(uint) => Ok(uint),
        Err(err) => Err(std::io::Error::new(std::io::ErrorKind::Other, err)),
    }
}
fn match_split(split: Option<&str>) -> Result<&str> {
    match split {
        Some(beg) => Ok(beg),
        None => Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Invalid .obj line",
        )),
    }
}
use smallvec::SmallVec;
use std::str::Split;
fn load_indices(split: &mut Split<&[char; 2]>) -> Result<(IndexType, IndexType, IndexType)> {
    Ok((
        parse_index(match_split(split.next())?)? - 1,
        parse_index(match_split(split.next())?)? - 1,
        parse_index(match_split(split.next())?)? - 1,
    ))
}
///IMPORTANT TODO: It seems likey that normals and uvs are spwapped in this function. Investigate and cleanup the confusion and refactor triangulation
fn load_face(split: &mut Split<&[char; 2]>, oi: &mut ModelImporter) -> Result<()> {
    let mut vertex_indices: SmallVec<[IndexType; SMALL_VEC_CAP]> = SmallVec::new();
    let mut uv_indices: SmallVec<[IndexType; SMALL_VEC_CAP]> = SmallVec::new();
    let mut normal_indices: SmallVec<[IndexType; SMALL_VEC_CAP]> = SmallVec::new();
    while let Ok((vertex_index, uv_index, normal_index)) = load_indices(split) {
        vertex_indices.push(vertex_index);
        uv_indices.push(uv_index);
        normal_indices.push(normal_index);
    }
    match oi.push_face(&vertex_indices, &uv_indices, &normal_indices) {
        Ok(()) => Ok(()),
        Err(msg) => Err(Error::new(ErrorKind::Other, msg)),
    }
}
pub fn load_vec3(split: &mut Split<&[char; 2]>) -> Result<Vector3> {
    let (x, y, z) = (
        match_split(split.next())?,
        match_split(split.next())?,
        match_split(split.next())?,
    );
    Ok((
        parse_float_type(x)?,
        parse_float_type(y)?,
        parse_float_type(z)?,
    ))
}
pub fn load_vec2(split: &mut Split<&[char; 2]>) -> Result<Vector2> {
    let (x, y) = (match_split(split.next())?, match_split(split.next())?);
    Ok((parse_float_type(x)?, parse_float_type(y)?))
}
fn save_obj<W: Write>(
    w: &mut W,
    mesh: &TMFMesh,
    index_offset: (IndexType, IndexType, IndexType),
    name: &str,
) -> Result<(IndexType, IndexType, IndexType)> {
    writeln!(w, "o {name}")?;
    let vertex_count = match mesh.get_vertices() {
        None => 0,
        Some(vertices) => {
            for vertex in vertices {
                writeln!(w, "v {} {} {}", vertex.0, vertex.1, vertex.2)?;
            }
            vertices.len()
        }
    };
    match mesh.get_normals() {
        None => (),
        Some(normals) => {
            for normal in normals {
                writeln!(w, "vn {} {} {}", normal.0, normal.1, normal.2)?;
            }
        }
    }
    match mesh.get_uvs() {
        None => (),
        Some(uvs) => {
            for uv in uvs {
                writeln!(w, "vt {} {}", uv.0, uv.1)?;
            }
        }
    }
    // Check triangle arrays
    // Get the length of the vertex triangle array to use for later array size comparison
    let vert_triangle_len = match mesh.get_vertex_triangles() {
        Some(vertex_triangles) => vertex_triangles.len(),
        // If no vertex triangle array, then object is a point cloud, so should not have any other triangle array.
        None => {
            if mesh.get_normal_triangles().is_some() {
                return Err(Error::new(ErrorKind::Other, "If vertex triangles data is not present, normal triangle data must not be present either!"));
            }
            if mesh.get_uv_triangles().is_some() {
                return Err(Error::new(
                    ErrorKind::Other,
                    "If vertex triangles data is not present, uv triangle data must not be present either!",
                ));
            }
            return Ok((0, 0, 0));
        }
    };
    // Ensure normal triangle array, if present, has the right length.
    match mesh.get_normal_triangles() {
        Some(normal_triangles) => {
            if normal_triangles.len() != vert_triangle_len {
                return Err(Error::new(
                    ErrorKind::Other,
                    "Number of triangles in the vertex triangle and normal triangle array differs.",
                ));
            }
        }
        None => (),
    }
    // Ensure uv triangle array, if present, has the right length.
    match mesh.get_uv_triangles() {
        Some(uv_triangles) => {
            if uv_triangles.len() != vert_triangle_len {
                return Err(Error::new(
                    ErrorKind::Other,
                    "Number of triangles in the vertex triangle and uv triangle array differs.",
                ));
            }
        }
        None => (),
    }
    // TODO: this part can be rewritten to be more efficient by checking if arrays are present beforehand.
    for i in 0..vert_triangle_len {
        if i % 3 == 0 {
            write!(w, "f ")?
        };
        // Why unwrap? Vertex triangle array MUST be present at this point in time, because if it was not, this function would have already returned.
        let vertex = mesh.get_vertex_triangles().unwrap()[i] + index_offset.0;
        write!(w, "{}", vertex)?;
        let normals = mesh.get_normal_triangles();
        match mesh.get_uv_triangles() {
            Some(uvs) => write!(w, "/{}", uvs[i] + index_offset.1)?,
            None => {
                if normals.is_some() {
                    write!(w, "/")?
                }
            }
        }
        match normals {
            Some(normals) => write!(w, "/{}", normals[i] + index_offset.2)?,
            None => (),
        }
        if i % 3 == 2 {
            writeln!(w)?
        } else {
            write!(w, " ")?
        }
    }
    let normal_count = match mesh.get_normals() {
        Some(normals) => normals.len(),
        None => 0,
    };
    let uv_count = match mesh.get_uvs() {
        Some(uvs) => uvs.len(),
        None => 0,
    };
    // If no vertices present, then no vertex triangles SHOULD be present, so if they are present, it is an error.
    Ok((
        vertex_count as IndexType,
        uv_count as IndexType,
        normal_count as IndexType,
    ))
}
/// Writes this TMF  mesh to a .obj file.
pub fn write_obj<W: Write, S: std::borrow::Borrow<str>>(
    meshes: &[(TMFMesh, S)],
    w: &mut W,
) -> Result<()> {
    let mut w = BufWriter::new(w);
    writeln!(w, "s 1")?;
    let mut index_offsets = (1, 1, 1);
    for (mesh, name) in meshes {
        let curr_offsets = save_obj(&mut w, mesh, index_offsets, name.borrow())?;
        index_offsets.0 += curr_offsets.0;
        index_offsets.1 += curr_offsets.1;
        index_offsets.2 += curr_offsets.2;
    }
    w.flush()?;
    Ok(())
}
