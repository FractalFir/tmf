use crate::utilis::*;
use crate::{FloatType, IndexType, TMFMesh};
/// Enum representing the result of integrity check.
#[derive(Clone)]
pub enum TMFIntegrityStatus {
    /// Mesh has passed all verification steps
    Ok,
    /// Vertex array is not present despite being required(eg. Vertex Face array is present);
    VertexArrayMissing,
    /// An *index* is larger than length of array *length*
    IndexOutsideVertexArray(IndexType, IndexType),
    /// Normal  array is not present despite being required(eg. Normal Face array is present);
    NormalArrayMissing,
    // Normals not normalzed
    NormalsNotNormalized,
    /// An *index* is larger than length of array *length*
    IndexOutsideNormalArray(IndexType, IndexType),
    /// UV corrds outside range
    UVOutsideRange(FloatType, FloatType),
    /// UV array is not present despite being required(eg. UV Face array is present);
    UVArrayMissing,
    /// An *index* is larger than length of array *length*
    IndexOutsideUVArray(IndexType, IndexType),
    /// More than one error
    CompositeError(Box<[Self]>),
}
//TODO: Redo this to work properly(wrong panic location).
impl TMFIntegrityStatus {
    /// Works like unwrap on [`Result`](panics if not Ok)
    pub fn unwrap(&self) {
        match self {
            Self::Ok => (),
            _ => panic!("{self}"),
        }
    }
    /// Works like execpt on [`Result`](panics with msg if not Ok)
    pub fn except(&self, msg: &str) {
        match self {
            Self::Ok => (),
            _ => panic!("{msg}:{self}"),
        }
    }
    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Ok)
    }
    pub fn is_err(&self) -> bool {
        !matches!(self, Self::Ok)
    }
}
impl std::fmt::Display for TMFIntegrityStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ok => write!(f, "Ok"),
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
            Self::CompositeError(errors) => {
                write!(f, "MultipleErrors{{")?;
                for err in errors.iter() {
                    write!(f, ",{err}")?;
                }
                write!(f, "}}")
            }
        }
    }
}
fn verify_vertices(mesh: &TMFMesh) -> TMFIntegrityStatus {
    match mesh.get_vertices() {
        None => {
            if mesh.get_vertex_faces().is_some() {
                TMFIntegrityStatus::VertexArrayMissing
            } else {
                TMFIntegrityStatus::Ok
            }
        }
        Some(vertices) => match mesh.get_vertex_faces() {
            Some(faces) => {
                for index in faces {
                    if *index > vertices.len() as IndexType {
                        return TMFIntegrityStatus::IndexOutsideVertexArray(
                            *index,
                            vertices.len() as IndexType,
                        );
                    }
                }
                TMFIntegrityStatus::Ok
            }
            None => TMFIntegrityStatus::Ok,
        },
    }
}
fn verify_uvs(mesh: &TMFMesh) -> TMFIntegrityStatus {
    match mesh.get_uvs() {
        None => {
            if mesh.get_uv_faces().is_some() {
                TMFIntegrityStatus::UVArrayMissing
            } else {
                TMFIntegrityStatus::Ok
            }
        }
        Some(uvs) => {
            for uv in uvs {
                if uv.0 < 0.0 && uv.1 < 0.0 && uv.0 > 1.0 && uv.1 > 1.0 {
                    return TMFIntegrityStatus::UVOutsideRange(uv.0, uv.1);
                }
            }
            match mesh.get_uv_faces() {
                Some(faces) => {
                    for index in faces {
                        if *index > uvs.len() as IndexType {
                            return TMFIntegrityStatus::IndexOutsideUVArray(
                                *index,
                                uvs.len() as IndexType,
                            );
                        }
                    }
                    TMFIntegrityStatus::Ok
                }
                None => TMFIntegrityStatus::Ok,
            }
        }
    }
}
//TODO: handle multiple errors
fn verify_normals(mesh: &TMFMesh) -> TMFIntegrityStatus {
    match mesh.get_normals() {
        None => {
            if mesh.get_normal_faces().is_some() {
                TMFIntegrityStatus::NormalArrayMissing
            } else {
                TMFIntegrityStatus::Ok
            }
        }
        Some(normals) => {
            for normal in normals {
                if (1.0 - magnitude(*normal)).abs() > 0.001 {
                    return TMFIntegrityStatus::NormalsNotNormalized;
                }
            }
            match mesh.get_normal_faces() {
                Some(faces) => {
                    for index in faces {
                        if *index > normals.len() as IndexType {
                            return TMFIntegrityStatus::IndexOutsideNormalArray(
                                *index,
                                normals.len() as IndexType,
                            );
                        }
                    }
                    TMFIntegrityStatus::Ok
                }
                None => TMFIntegrityStatus::Ok,
            }
        }
    }
}
pub(crate) fn verify_tmf_mesh(mesh: &TMFMesh) -> TMFIntegrityStatus {
    let mut errors = Vec::new();
    // Verify vertices
    let vertices_status = verify_vertices(mesh);
    if vertices_status.is_err() {
        errors.push(vertices_status)
    };
    //Verify normals
    let normals_status = verify_normals(mesh);
    if normals_status.is_err() {
        errors.push(normals_status)
    };
    //Verify uvs
    let uvs_status = verify_uvs(mesh);
    if uvs_status.is_err() {
        errors.push(uvs_status)
    };
    let len = errors.len();
    match len {
        0 => TMFIntegrityStatus::Ok,
        1 => errors[0].clone(),
        _ => TMFIntegrityStatus::CompositeError(errors.into()),
    }
}
