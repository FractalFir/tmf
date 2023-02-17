use crate::utilis::*;
use crate::{FloatType, IndexType, TMFMesh};
/// Enum representing the result of integrity check.
#[derive(Clone)]
pub enum TMFIntegrityStatus {
    /// Vertex array is not present despite being required(eg. Vertex triangle array is present);
    VertexArrayMissing,
    /// An *index* is larger than length of array *length*
    IndexOutsideVertexArray(IndexType, IndexType),
    /// Normal  array is not present despite being required(eg. Normal triangle array is present);
    NormalArrayMissing,
    /// Some or all normals in the normal array are not normalised.
    NormalsNotNormalized,
    /// An *index* is larger than length of array *length*
    IndexOutsideNormalArray(IndexType, IndexType),
    /// UV corrds outside range
    UVOutsideRange(FloatType, FloatType),
    /// UV array is not present despite being required(eg. UV triangle array is present);
    UVArrayMissing,
    /// An *index* is larger than length of array *length*
    IndexOutsideUVArray(IndexType, IndexType),
    /// More than one error
    CompositeError(Box<[Self]>),
}
fn write_composite_error(
    f: &mut std::fmt::Formatter<'_>,
    errors: &[TMFIntegrityStatus],
) -> std::fmt::Result {
    write!(f, "MultipleErrors{{")?;
    for err in errors.iter() {
        write!(f, ",{err}")?;
    }
    write!(f, "}}")
}
impl std::fmt::Display for TMFIntegrityStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::VertexArrayMissing => write!(f, "VertexArrayMissing"),
            Self::IndexOutsideVertexArray(index, length) => {
                write!(f, "IndexOutsideVertexArray{{index:{index},length{length}}}")
            }
            Self::NormalArrayMissing => write!(f, "NormalArrayMissing"),
            Self::NormalsNotNormalized => write!(f, "NormalsNotNormalized"),
            Self::IndexOutsideNormalArray(index, length) => {
                write!(f, "IndexOutsideNormalArray{{index:{index},length{length}}}")
            }
            Self::IndexOutsideUVArray(index, length) => {
                write!(f, "IndexOutsideUVArray{{index:{index},length{length}}}")
            }
            Self::UVOutsideRange(x, y) => write!(f, "UVOutsideRange{{x:{x},y:{y}}}"),
            Self::UVArrayMissing => write!(f, "UVArrayMissing"),
            Self::CompositeError(errors) => write_composite_error(f, errors),
        }
    }
}
impl std::fmt::Debug for TMFIntegrityStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        core::fmt::Display::fmt(self, f)
    }
}
/// Checks if indices fit inside the array of *length*, returned None if so or Some(index) if not.
fn indices_inside_array(array: &[IndexType], length: IndexType) -> Option<IndexType> {
    for index in array {
        if *index > length as IndexType {
            return Some(*index);
        }
    }
    None
}
fn verify_vertices(mesh: &TMFMesh) -> Result<(), TMFIntegrityStatus> {
    match mesh.get_vertices() {
        None => {
            if mesh.get_vertex_triangles().is_some() {
                Err(TMFIntegrityStatus::VertexArrayMissing)
            } else {
                Ok(())
            }
        }
        Some(vertices) => match mesh.get_vertex_triangles() {
            Some(triangles) => match indices_inside_array(triangles, vertices.len() as IndexType) {
                Some(index) => Err(TMFIntegrityStatus::IndexOutsideVertexArray(
                    index,
                    vertices.len() as IndexType,
                )),
                None => Ok(()),
            },
            None => Ok(()),
        },
    }
}
fn verify_uvs(mesh: &TMFMesh) -> Result<(), TMFIntegrityStatus> {
    match mesh.get_uvs() {
        None => {
            if mesh.get_uv_triangles().is_some() {
                Err(TMFIntegrityStatus::UVArrayMissing)
            } else {
                Ok(())
            }
        }
        Some(uvs) => {
            for uv in uvs {
                if uv.0 < 0.0 && uv.1 < 0.0 && uv.0 > 1.0 && uv.1 > 1.0 {
                    return Err(TMFIntegrityStatus::UVOutsideRange(uv.0, uv.1));
                }
            }
            match mesh.get_uv_triangles() {
                Some(triangles) => match indices_inside_array(triangles, uvs.len() as IndexType) {
                    Some(index) => Err(TMFIntegrityStatus::IndexOutsideUVArray(
                        index,
                        uvs.len() as IndexType,
                    )),
                    None => Ok(()),
                },
                None => Ok(()),
            }
        }
    }
}
//TODO: handle multiple errors
fn verify_normals(mesh: &TMFMesh) -> Result<(), TMFIntegrityStatus> {
    match mesh.get_normals() {
        None => {
            if mesh.get_normal_triangles().is_some() {
                Err(TMFIntegrityStatus::NormalArrayMissing)
            } else {
                Ok(())
            }
        }
        Some(normals) => {
            for normal in normals {
                if (1.0 - magnitude(*normal)).abs() > 0.001 {
                    return Err(TMFIntegrityStatus::NormalsNotNormalized);
                }
            }
            match mesh.get_normal_triangles() {
                Some(triangles) => match indices_inside_array(triangles, normals.len() as IndexType) {
                    Some(index) => Err(TMFIntegrityStatus::IndexOutsideNormalArray(
                        index,
                        normals.len() as IndexType,
                    )),
                    None => Ok(()),
                },
                None => Ok(()),
            }
        }
    }
}
pub(crate) fn verify_tmf_mesh(mesh: &TMFMesh) -> Result<(), TMFIntegrityStatus> {
    let mut errors = Vec::new();
    // Verify vertices
    let vertices_status = verify_vertices(mesh);
    match vertices_status {
        Err(err) => errors.push(err),
        Ok(_) => (),
    };
    //Verify normals
    let normals_status = verify_normals(mesh);
    match normals_status {
        Err(err) => errors.push(err),
        Ok(_) => (),
    };
    //Verify uvs
    let uvs_status = verify_uvs(mesh);
    match uvs_status {
        Err(err) => errors.push(err),
        Ok(_) => (),
    };
    let len = errors.len();
    match len {
        0 => Ok(()),
        1 => Err(errors[0].clone()),
        _ => Err(TMFIntegrityStatus::CompositeError(errors.into())),
    }
}
