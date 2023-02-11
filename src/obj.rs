use crate::{FloatType, IndexType, TMFMesh, Vector2, Vector3};
use std::io::{BufReader, BufWriter, Error, ErrorKind, Read, Result, Write};
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
        Err(err) => Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            err.to_string(),
        )),
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
pub fn load_face(
    split: &mut Split<&[char; 2]>,
    vertex_faces: &mut Vec<IndexType>,
    normal_faces: &mut Vec<IndexType>,
    uv_faces: &mut Vec<IndexType>,
) -> Result<()> {
    let mut faces: SmallVec<[(IndexType, IndexType, IndexType); 6]> = SmallVec::new();
    while let Ok(indices) = load_indices(split) {
        faces.push(indices);
    }
    assert!(faces.len() == 3,"Object loader currently supports only loading triangulated faces, but encountered a {} sided polygon!",faces.len());
    //TODO: do triangulation
    for face in faces {
        vertex_faces.push(face.0);
        normal_faces.push(face.2);
        uv_faces.push(face.1);
    }
    Ok(())
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
    // Check face arrays
    // Get the length of the vertex face array to use for later array size comparison
    let vert_face_len = match mesh.get_vertex_faces() {
        Some(vertex_faces) => vertex_faces.len(),
        // If no vertex face array, then object is a point cloud, so should not have any other face array.
        None => {
            if mesh.get_normal_faces().is_some() {
                return Err(Error::new(ErrorKind::Other, "If vertex faces data is not present, normal face data must not be present either!"));
            }
            if mesh.get_uv_faces().is_some() {
                return Err(Error::new(
                    ErrorKind::Other,
                    "If vertex faces data is not present, uv face data must not be present either!",
                ));
            }
            return Ok((0, 0, 0));
        }
    };
    // Ensure normal face array, if present, has the right length.
    match mesh.get_normal_faces() {
        Some(normal_faces) => {
            if normal_faces.len() != vert_face_len {
                return Err(Error::new(
                    ErrorKind::Other,
                    "Number of faces in the vertex face and normal face array differs.",
                ));
            }
        }
        None => (),
    }
    // Ensure uv face array, if present, has the right length.
    match mesh.get_uv_faces() {
        Some(uv_faces) => {
            if uv_faces.len() != vert_face_len {
                return Err(Error::new(
                    ErrorKind::Other,
                    "Number of faces in the vertex face and uv face array differs.",
                ));
            }
        }
        None => (),
    }
    // TODO: this part can be rewritten to be more efficient by checking if arrays are present beforehand.
    for i in 0..vert_face_len {
        if i % 3 == 0 {
            write!(w, "f ")?
        };
        // Why unwrap? Vertex face array MUST be present at this point in time, because if it was not, this function would have already returned.
        let vertex = mesh.get_vertex_faces().unwrap()[i] + index_offset.0;
        write!(w, "{}", vertex)?;
        let normals = mesh.get_normal_faces();
        match mesh.get_uv_faces() {
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
    // If no vertices present, then no vertex faces SHOULD be present, so if they are present, it is an error.
    Ok((
        vertex_count as IndexType,
        uv_count as IndexType,
        normal_count as IndexType,
    ))
}
// Stores information shared between objects.
struct ObjReadCtx {
    pub mtl_lib: String,
    pub mtl: String,
}
impl Default for ObjReadCtx {
    fn default() -> Self {
        Self {
            mtl_lib: "".to_owned(),
            mtl: "".to_owned(),
        }
    }
}
/// Returns the readen mesh and name of the next object if present
fn load_obj<R: std::io::BufRead>(
    lines: &mut std::io::Lines<R>,
    vertices: &mut Vec<Vector3>,
    normals: &mut Vec<Vector3>,
    uvs: &mut Vec<Vector2>,
    ctx: &mut ObjReadCtx,
) -> Result<(Option<TMFMesh>, Option<String>)> {
    // Prepare face data
    let mut vertex_faces = Vec::with_capacity(0x100);
    let mut normal_faces = Vec::with_capacity(0x100);
    let mut uv_faces = Vec::with_capacity(0x100);
    let mut materials:Vec<String> = Vec::new();
    let mut last_mtl_face_index = 0;
    // Iterate over all lines in input to parse them
    for line in lines {
        // Check that line is properly readen
        let line = line?;
        // Split the line by white spaces and '/' sign used in faces
        let mut split = line.split(&[' ', '/']);
        // Get the beginning of the line
        let beg = match_split(split.next())?;
        match beg {
            "#" => (), //Ignore comments
            "mtllib" => {
                let lib = match_split(split.next())?;
                ctx.mtl_lib = lib.to_owned();
                ctx.mtl = "".to_owned();
            }
            "usemtl" => {
                 // If a material is in use  AND there have been some faces since last mtl push
                if (ctx.mtl != "" || ctx.mtl_lib != "") && last_mtl_face_index < vertex_faces.len() {
                    println!("pushing mtl {} in lib {}",ctx.mtl,ctx.mtl_lib);
                    last_mtl_face_index = (vertex_faces.len() - 1);
                    let mtl_name = ctx.mtl_lib.to_owned() + "/" + &ctx.mtl;
                    // Index of the found material
                    let mut index = 0;
                     println!("mtl_name:{mtl_name}");
                }
                let mtl = match_split(split.next())?;
                ctx.mtl = mtl.to_owned();
            },
            "s" => (),      //Ignore smothnes info
            "v" => {
                vertices.push(load_vec3(&mut split)?);
            }
            "vn" => {
                normals.push(load_vec3(&mut split)?);
            }
            "vt" => {
                let (x, y) = (match_split(split.next())?, match_split(split.next())?);
                let uv = (parse_float_type(x)?, parse_float_type(y)?);
                uvs.push(uv);
            }
            "f" => {
                load_face(
                    &mut split,
                    &mut vertex_faces,
                    &mut normal_faces,
                    &mut uv_faces,
                )?;
            }
            "o" => {
                let name = match_split(split.next())?;
                if vertices.len() > 0 {
                    let mut res = TMFMesh::empty();
                    // Needed to remove some vertices which do not belong to this mesh.
                    let mut new_vertices = vertices.clone();
                    crate::utilis::fast_prune(&mut new_vertices, &mut vertex_faces);
                    // Needed to remove some uvs which do not belong to this mesh.
                    let mut new_uvs = uvs.clone();
                    crate::utilis::fast_prune(&mut new_uvs, &mut uv_faces);
                    // Needed to remove some normals which do not belong to this mesh.
                    let mut new_normals = normals.clone();
                    crate::utilis::fast_prune(&mut new_normals, &mut normal_faces);
                    // Set mesh data
                    res.set_vertices(&new_vertices);
                    res.set_normals(&new_normals);
                    res.set_uvs(&new_uvs);
                    res.set_vertex_faces(&vertex_faces);
                    res.set_normal_faces(&normal_faces);
                    res.set_uv_faces(&uv_faces);
                    return Ok((Some(res), Some(name.to_owned())));
                }
                return Ok((None, Some(name.to_owned())));
            }
            _ => todo!("unhandled line '{beg}'"),
        }
    }
    if vertices.len() > 0 {
        let mut res = TMFMesh::empty();
        res.set_vertices(&vertices);
        res.set_normals(&normals);
        res.set_uvs(&uvs);
        res.set_vertex_faces(&vertex_faces);
        res.set_normal_faces(&normal_faces);
        res.set_uv_faces(&uv_faces);
        Ok((Some(res), None))
    } else {
        Ok((None, None))
    }
}
pub fn read_from_obj<R: Read>(reader: &mut R) -> Result<Vec<(TMFMesh, String)>> {
    use std::io::BufRead;
    let reader = BufReader::new(reader);
    let mut vertices = Vec::with_capacity(0x100);
    let mut normals = Vec::with_capacity(0x100);
    let mut uvs = Vec::with_capacity(0x100);

    let mut lines = reader.lines();
    let mut name: Option<String> = None;
    let mut res = Vec::new();
    let mut ctx = ObjReadCtx::default();
    loop {
        let (curr, curr_name) =
            load_obj(&mut lines, &mut vertices, &mut normals, &mut uvs, &mut ctx)?;
        if curr.is_some() {
            // TODO: find way to remove unnecessary clone call
            res.push((curr.unwrap(), name.clone().unwrap_or("".to_owned())));
        }
        if !curr_name.is_some() {
            break;
        }
        name = curr_name;
    }
    Ok(res)
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
