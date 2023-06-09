use crate::IndexType;
fn reorder_triangles<const ARRAY_COUNT: usize>(
    triangles: &[&[IndexType]],
) -> Box<[Box<[IndexType]>]> {
    assert_eq!(triangles.len(), ARRAY_COUNT);
    let len = triangles[0].len();
    for array in triangles {
        assert_eq!(array.len(), len);
    }
    let mut max_deltas = Vec::with_capacity(len);
    let mut last: [IndexType; ARRAY_COUNT] = [0; ARRAY_COUNT];
    for index in 0..len {
        let mut delta = 0;
        for array_index in 0..ARRAY_COUNT {
            let curr = triangles[array_index][index];
            delta = delta.max((curr as isize - last[array_index] as isize).unsigned_abs());
            last[array_index] = curr;
        }
        max_deltas.push(delta);
    }
    // let mut tri_deltas = Vec::with_capacity(len/3);
    let mut res_sorted = Vec::with_capacity(len);
    let mut res_unorganized = Vec::with_capacity(len);
    for index in 0..(len / 3) {
        let max = max_deltas[index * 3]
            .max(max_deltas[index * 3 + 1])
            .max(max_deltas[index * 3 + 2]);
        //tri_deltas.push(max);
        let mut value: [[IndexType; ARRAY_COUNT]; 3] = [[0; ARRAY_COUNT]; 3]; //
        for array_index in 0..ARRAY_COUNT {
            value[0][array_index] = triangles[array_index][index * 3];
            value[1][array_index] = triangles[array_index][index * 3 + 1];
            value[2][array_index] = triangles[array_index][index * 3 + 2];
        }
        if max < 16 {
            res_sorted.extend(value);
        } else {
            res_unorganized.extend(value);
        }
        //println!("delta:{max}");
    }
    let mut new_indices: Vec<Box<[_]>> = vec![[][..].into(); ARRAY_COUNT];
    for array_index in 0..ARRAY_COUNT {
        let mut new_index_array: Vec<_> = res_sorted
            .iter()
            .map(|arrays| arrays[array_index])
            .collect();
        new_index_array.extend(res_unorganized.iter().map(|arrays| arrays[array_index]));
        let new_index_array: Box<[_]> = new_index_array.into();
        new_indices[array_index] = new_index_array;
    }
    println!("{len} {}", new_indices[0].len());
    new_indices.into()
}
#[test]
#[cfg(feature = "obj_import")]
fn read_susan_obj() {
    use crate::init_test_env;
    use crate::{TMFMesh, TMFPrecisionInfo};
    init_test_env();
    let mut file = std::fs::File::open("testing/susan.obj").unwrap();
    let mut tmf_mesh = TMFMesh::read_from_obj_one(&mut file).unwrap().0;
    tmf_mesh.verify().unwrap();
    tmf_mesh.reorder_data();
    tmf_mesh.unify_index_data();
    tmf_mesh.verify().unwrap();
    let indices = reorder_triangles::<3>(&[
        tmf_mesh.get_vertex_triangles().unwrap(),
        tmf_mesh.get_normal_triangles().unwrap(),
        tmf_mesh.get_uv_triangles().unwrap(),
    ]);
    let (vertex_indices, normal_indices, uv_indices) =
        (indices[0].clone(), indices[1].clone(), indices[2].clone());
    assert_eq!(
        vertex_indices.len(),
        tmf_mesh.get_vertex_triangles().unwrap().len()
    );
    tmf_mesh.set_vertex_triangles(vertex_indices);
    tmf_mesh.set_normal_triangles(normal_indices);
    tmf_mesh.set_uv_triangles(uv_indices);
    let mut out = std::fs::File::create("target/test_res/suan_reordered_ftmf.obj").unwrap();
    tmf_mesh.write_obj_one(&mut out, &"SUSAN").unwrap();
    let mut out = std::fs::File::create("target/test_res/susan_reordered.tmf").unwrap();
    tmf_mesh
        .write_tmf_one(&mut out, &TMFPrecisionInfo::default(), "SUSAN")
        .unwrap();

    //todo!();
}
