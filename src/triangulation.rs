#[cfg(not(feature = "model_importer"))]
compile_error!("Feature \"triangulation\" is only useful when model importer feature is enabled, and is useless, dead code otherwise.");
use crate::utilis::{self, add_vec3, magnitude, normalize, sub_vec3};
use crate::{FloatType, IndexType, Vector3};
// angle inside triangle in radians
fn compute_angle(prev: Vector3, curr: Vector3, next: Vector3) -> FloatType {
    let cp = normalize((curr.0 - prev.0, curr.1 - prev.1, curr.2 - prev.2));
    let cn = normalize((curr.0 - next.0, curr.1 - next.1, curr.2 - next.2));
    let dot = utilis::dot(cp, cn);
    //Vectors are normalised, so there is no point dividing them by magnitude
    FloatType::acos(dot)
}
// Still has issues
fn is_point_inside_poly(
    point: Vector3,
    vertices: &[Vector3],
    indices: &[(IndexType, IndexType, IndexType)],
) -> bool {
    /*
    let mut up = (0.0,0.0,0.0);
    use utilis::*;
    for index in 0..indices.len(){
         let next_index = index.wrapping_add(1)%indices.len();
         let dir = normalize(sub_vec3(vertices[indices[index].0],vertices[indices[next_index].0]));
         up += cross(dir,(0.0,1.0,0.0));
    }
    let ray_shot_dir = cross(up,(0.0,1.0,0.0));
    */
    let mut sum = (0.0, 0.0, 0.0);
    for index in 0..indices.len() {
        let dir = normalize(sub_vec3(vertices[indices[index].0 as usize], point));
        sum = add_vec3(dir, sum);
    }
    let avg_mag = magnitude(sum) / (indices.len() as FloatType);
    avg_mag < 1.0
}
fn is_line_inside_poly(
    a: Vector3,
    b: Vector3,
    vertices: &[Vector3],
    indices: &[(IndexType, IndexType, IndexType)],
) -> bool {
    //TODO:evaluate if line is inside a polygon
    let middle = add_vec3(a, b);
    let middle = (middle.0 / 2.0, middle.1 / 2.0, middle.2 / 2.0);
    is_point_inside_poly(middle, vertices, indices)
}
fn eval_ear(
    tri: (usize, usize, usize),
    vertices: &[Vector3],
    indices: &[(IndexType, IndexType, IndexType)],
) -> FloatType {
    let angle = compute_angle(vertices[tri.0], vertices[tri.1], vertices[tri.2]);
    let weight = if is_line_inside_poly(vertices[tri.0], vertices[tri.2], vertices, indices) {
        angle
    } else {
        -angle
    };
    weight
}
use smallvec::SmallVec;
fn calc_ear_val_at_index(
    indices: &[(IndexType, IndexType, IndexType)],
    index: usize,
    vertices: &[Vector3],
) -> FloatType {
    let tri = (
        index.wrapping_sub(1) % indices.len(),
        index,
        index.wrapping_add(1) % indices.len(),
    );
    let p0 = indices[tri.0].0 as usize;
    let p1 = indices[tri.1].0 as usize;
    let p2 = indices[tri.2].0 as usize;
    let tri = (p0, p1, p2);
    return eval_ear(tri, vertices, indices);
}
pub fn triangulate(
    mut indices: SmallVec<[(IndexType, IndexType, IndexType); 6]>,
    vertex_triangles: &mut Vec<IndexType>,
    normal_triangles: &mut Vec<IndexType>,
    uv_triangles: &mut Vec<IndexType>,
    vertices: &[Vector3],
) {
    let mut ear_vals: Vec<FloatType> = Vec::with_capacity(indices.len());
    for index in 0_usize..indices.len() {
        ear_vals.push(calc_ear_val_at_index(&indices, index, vertices));
    }
    while indices.len() > 3 {
        let mut best_index = 0;
        for index in 0_usize..indices.len() {
            if ear_vals[index] > ear_vals[best_index] {
                best_index = index;
            }
        }
        let prev_index = best_index.wrapping_sub(1) % indices.len();
        let curr_index = best_index;
        let next_index = best_index.wrapping_add(1) % indices.len();
        vertex_triangles.push(indices[prev_index].0);
        vertex_triangles.push(indices[curr_index].0);
        vertex_triangles.push(indices[next_index].0);
        normal_triangles.push(indices[prev_index].2);
        normal_triangles.push(indices[curr_index].2);
        normal_triangles.push(indices[next_index].2);
        uv_triangles.push(indices[prev_index].1);
        uv_triangles.push(indices[curr_index].1);
        uv_triangles.push(indices[next_index].1);
        indices.remove(curr_index);
        ear_vals.remove(curr_index);
        let curr_index = curr_index % indices.len();
        let prev_index = best_index.wrapping_sub(1) % indices.len();
        ear_vals[curr_index] = calc_ear_val_at_index(&indices, curr_index, vertices);
        ear_vals[prev_index] = calc_ear_val_at_index(&indices, prev_index, vertices);
    }
    vertex_triangles.push(indices[0].0);
    vertex_triangles.push(indices[1].0);
    vertex_triangles.push(indices[2].0);
    normal_triangles.push(indices[0].2);
    normal_triangles.push(indices[1].2);
    normal_triangles.push(indices[2].2);
    uv_triangles.push(indices[0].1);
    uv_triangles.push(indices[1].1);
    uv_triangles.push(indices[2].1);
}
