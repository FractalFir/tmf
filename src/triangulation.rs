#[cfg(not(feature = "model_importer"))]
compile_error!("Feature \"triangulation\" is only useful when model importer feature is enabled, and is useless, dead code otherwise.");
use crate::utilis::{self, add_vec3, magnitude, normalize, sub_vec3};
use crate::{FloatType};
// angle inside triangle in radians
fn compute_angle(prev: Vector3, curr: Vector3, next: Vector3) -> FloatType {
    let cp = normalize((curr.0 - prev.0, curr.1 - prev.1, curr.2 - prev.2));
    let cn = normalize((curr.0 - next.0, curr.1 - next.1, curr.2 - next.2));
    let dot = utilis::dot(cp, cn);
    //Vectors are normalised, so there is no point dividing them by magnitude
    FloatType::acos(dot)
}
fn eval_ear(
    tri: (usize, usize, usize),
    vertices: &[Vector3],
    indices: &[IndexType],
) -> FloatType {
    let angle = compute_angle(vertices[tri.0], vertices[tri.1], vertices[tri.2]);
    let weight = if is_line_inside_poly(vertices[tri.0], vertices[tri.2], vertices, indices) {
        angle
    } else {
        -angle
    };
    weight
}
fn is_line_inside_poly(
    a: Vector3,
    b: Vector3,
    vertices: &[Vector3],
    indices: &[IndexType],
) -> bool {
    true
}
fn calc_ear_val_at_index(
    indices: &[IndexType],
    index: usize,
    vertices: &[Vector3],
) -> FloatType {
    let tri = (
        index.wrapping_sub(1) % indices.len(),
        index,
        index.wrapping_add(1) % indices.len(),
    );
    let p0 = indices[tri.0] as usize;
    let p1 = indices[tri.1] as usize;
    let p2 = indices[tri.2] as usize;
    let tri = (p0, p1, p2);
    return eval_ear(tri, vertices, indices);
}
use smallvec::SmallVec;
fn triangulate(
    mi:&mut ModelImporter,
    mut vertex_indices:SmallVec<[IndexType; SMALL_VEC_CAP]>,
    mut normal_indices:SmallVec<[IndexType; SMALL_VEC_CAP]>,
    mut uv_indices:SmallVec<[IndexType; SMALL_VEC_CAP]>,
) {
    debug_assert!(vertex_indices.len() == normal_indices.len()&& vertex_indices.len() == uv_indices.len());
    let mut ear_vals: Vec<FloatType> = Vec::with_capacity(vertex_indices.len());
    for index in 0_usize..vertex_indices.len() {
        ear_vals.push(calc_ear_val_at_index(&vertex_indices, index, &mi.vertices));
    }
    while vertex_indices.len() > 3 {
        let mut best_index = 0;
        for index in 0_usize..vertex_indices.len() {
            if ear_vals[index] > ear_vals[best_index] {
                best_index = index;
            }
        }
        let prev_index = best_index.wrapping_sub(1) % vertex_indices.len();
        let curr_index = best_index;
        let next_index = best_index.wrapping_add(1) % vertex_indices.len();
        
        mi.push_vertex_index(vertex_indices[prev_index]);
        mi.push_vertex_index(vertex_indices[curr_index]);
        mi.push_vertex_index(vertex_indices[next_index]);
        
        mi.push_normal_index(normal_indices[prev_index]);
        mi.push_normal_index(normal_indices[curr_index]);
        mi.push_normal_index(normal_indices[next_index]);
        
        mi.push_uv_index(uv_indices[prev_index]);
        mi.push_uv_index(uv_indices[curr_index]);
        mi.push_uv_index(uv_indices[next_index]);
        
        vertex_indices.remove(curr_index);
        normal_indices.remove(curr_index);
        uv_indices.remove(curr_index);
        
        ear_vals.remove(curr_index);
        
        let curr_index = curr_index % vertex_indices.len();
        let prev_index = best_index.wrapping_sub(1) % vertex_indices.len();
        ear_vals[curr_index] = calc_ear_val_at_index(&vertex_indices, curr_index, &mi.vertices);
        ear_vals[prev_index] = calc_ear_val_at_index(&vertex_indices, prev_index, &mi.vertices);
        
    }
    mi.push_vertex_index(vertex_indices[0]);
    mi.push_vertex_index(vertex_indices[1]);
    mi.push_vertex_index(vertex_indices[2]);
        
    mi.push_normal_index(normal_indices[0]);
    mi.push_normal_index(normal_indices[1]);
    mi.push_normal_index(normal_indices[2]);
        
    mi.push_uv_index(uv_indices[0]);
    mi.push_uv_index(uv_indices[1]);
    mi.push_uv_index(uv_indices[2]);
}


