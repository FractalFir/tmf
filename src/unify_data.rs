#![allow(clippy::type_complexity)]
use crate::IndexType;
use std::collections::HashMap;
/// This function unifies data at index level, returning info necessary for merging to `merge_data`.
fn unfiy_data_common<const DATA_COUNT: usize>(
    indices: &[&[IndexType]],
) -> (Box<[IndexType]>, Box<[Box<[IndexType]>]>) {
    type ComboType<const DATA_COUNT: usize> = [IndexType; DATA_COUNT];
    let mut combos: HashMap<ComboType<DATA_COUNT>, usize> = HashMap::with_capacity(indices.len());
    assert_eq!(DATA_COUNT, indices.len());
    assert!(!indices.is_empty());
    let len = indices[0].len();
    for index_arr in indices {
        assert_eq!(len, index_arr.len());
    }
    let mut combo_id = 0;
    let mut new_indices = Vec::with_capacity(indices.len());
    for index in 0..len {
        let mut curr_combo: ComboType<DATA_COUNT> = [0; DATA_COUNT];
        for sub_index in 0..DATA_COUNT {
            curr_combo[sub_index] = indices[sub_index][index];
        }
        let curr_id = *combos.entry(curr_combo).or_insert_with(|| {
            let id = combo_id;
            combo_id += 1;
            id
        });
        new_indices.push(curr_id as IndexType);
        //println!("curr_combo:{curr_combo:?}");
    }
    let mut combos = combos.iter().collect::<Vec<_>>();
    combos.sort_by(|a, b| a.1.cmp(b.1));
    let combos: Box<[_]> = combos.iter().map(|(combo, _id)| combo).collect();
    let mut data_map: Box<[_]> = vec![Vec::with_capacity(len); DATA_COUNT].into();
    for combo in combos.iter() {
        for i in 0..DATA_COUNT {
            data_map[i].push(combo[i]);
        }
    }
    let data_map: Box<[Box<[_]>]> = data_map.iter().map(|v| (*v).clone().into()).collect();
    (new_indices.into(), data_map)
}
pub(crate) fn merge_data_2<A: Copy, B: Copy>(
    indices: &[&[IndexType]; 2],
    a: &[A],
    b: &[B],
) -> (Box<[IndexType]>, Box<[A]>, Box<[B]>) {
    let (indices, index_map) = unfiy_data_common::<2>(indices);
    let a: Box<[_]> = index_map[0]
        .iter()
        .map(|index| a[*index as usize])
        .collect();
    let b: Box<[_]> = index_map[1]
        .iter()
        .map(|index| b[*index as usize])
        .collect();
    (indices, a, b)
}
pub(crate) fn merge_data_3<A: Copy, B: Copy, C: Copy>(
    indices: &[&[IndexType]; 3],
    a: &[A],
    b: &[B],
    c: &[C],
) -> (Box<[IndexType]>, Box<[A]>, Box<[B]>, Box<[C]>) {
    let (indices, index_map) = unfiy_data_common::<3>(indices);
    let a: Box<[_]> = index_map[0]
        .iter()
        .map(|index| a[*index as usize])
        .collect();
    let b: Box<[_]> = index_map[1]
        .iter()
        .map(|index| b[*index as usize])
        .collect();
    let c: Box<[_]> = index_map[2]
        .iter()
        .map(|index| c[*index as usize])
        .collect();
    (indices, a, b, c)
}
pub(crate) fn merge_data_4<A: Copy, B: Copy, C: Copy, D: Copy>(
    indices: &[&[IndexType]; 4],
    a: &[A],
    b: &[B],
    c: &[C],
    d: &[D],
) -> (Box<[IndexType]>, Box<[A]>, Box<[B]>, Box<[C]>, Box<[D]>) {
    let (indices, index_map) = unfiy_data_common::<4>(indices);
    let a: Box<[_]> = index_map[0]
        .iter()
        .map(|index| a[*index as usize])
        .collect();
    let b: Box<[_]> = index_map[1]
        .iter()
        .map(|index| b[*index as usize])
        .collect();
    let c: Box<[_]> = index_map[2]
        .iter()
        .map(|index| c[*index as usize])
        .collect();
    let d: Box<[_]> = index_map[3]
        .iter()
        .map(|index| d[*index as usize])
        .collect();
    (indices, a, b, c, d)
}
type OBoxArr<A> = Option<Box<[A]>>;
fn is_merge_needed(indices: &[Option<&[IndexType]>]) -> bool {
    // filter keeps only `Some` values
    let mut index_iter = indices.iter().filter_map(|x| x.as_ref());
    if let Some(first) = index_iter.next() {
        !index_iter.all(|array| *array == *first)
    } else {
        false
    }
}
fn smart_merge_data_2<A: Copy, B: Copy>(
    a: Option<&[A]>,
    b: Option<&[B]>,
    indices: [Option<&[IndexType]>; 2],
) -> (OBoxArr<A>, OBoxArr<B>, OBoxArr<IndexType>) {
    if !is_merge_needed(&indices) {
        return (None, None, None);
    }
    if let Some(((a, indices_a), (b, indices_b))) = a.zip(indices[0]).zip(b.zip(indices[1])) {
        let (indices, a, b) = merge_data_2(&[indices_a, indices_b], a, b);
        (Some(a), Some(b), Some(indices))
    } else {
        (None, None, None)
    }
}
pub(crate) fn smart_merge_data_3<A: Copy, B: Copy, C: Copy>(
    a: Option<&[A]>,
    b: Option<&[B]>,
    c: Option<&[C]>,
    indices: [Option<&[IndexType]>; 3],
) -> (OBoxArr<A>, OBoxArr<B>, OBoxArr<C>, OBoxArr<IndexType>) {
    if !is_merge_needed(&indices) {
        return (None, None, None, None);
    }
    if !a.is_some_and(|data| !data.is_empty()) || indices[0].is_none() {
        let (b, c, indices) = smart_merge_data_2(b, c, [indices[1], indices[2]]);
        (None, b, c, indices)
    } else if !b.is_some_and(|data| !data.is_empty()) || indices[1].is_none() {
        let (a, c, indices) = smart_merge_data_2(a, c, [indices[0], indices[2]]);
        (a, None, c, indices)
    } else if !c.is_some_and(|data| !data.is_empty()) || indices[2].is_none() {
        let (a, b, indices) = smart_merge_data_2(a, b, [indices[0], indices[1]]);
        (a, b, None, indices)
    } else if let Some((((a, indices_a), (b, indices_b)), (c, indices_c))) = a
        .zip(indices[0])
        .zip(b.zip(indices[1]))
        .zip(c.zip(indices[2]))
    {
        let (indices, a, b, c) = merge_data_3(&[indices_a, indices_b, indices_c], a, b, c);
        (Some(a), Some(b), Some(c), Some(indices))
    } else {
        (None, None, None, None)
    }
}
pub(crate) fn smart_merge_data_4<A: Copy, B: Copy, C: Copy, D: Copy>(
    a: Option<&[A]>,
    b: Option<&[B]>,
    c: Option<&[C]>,
    d: Option<&[D]>,
    indices: [Option<&[IndexType]>; 4],
) -> (
    OBoxArr<A>,
    OBoxArr<B>,
    OBoxArr<C>,
    OBoxArr<D>,
    OBoxArr<IndexType>,
) {
    if !is_merge_needed(&indices) {
        return (None, None, None, None, None);
    }
    if !a.is_some_and(|data| !data.is_empty()) || indices[0].is_none() {
        let (b, c, d, indices) = smart_merge_data_3(b, c, d, [indices[1], indices[2], indices[3]]);
        (None, b, c, d, indices)
    } else if !b.is_some_and(|data| !data.is_empty()) || indices[1].is_none() {
        let (a, c, d, indices) = smart_merge_data_3(a, c, d, [indices[0], indices[2], indices[3]]);
        (a, None, c, d, indices)
    } else if !c.is_some_and(|data| !data.is_empty()) || indices[2].is_none() {
        let (a, b, d, indices) = smart_merge_data_3(a, b, d, [indices[0], indices[1], indices[3]]);
        (a, b, None, d, indices)
    } else if !d.is_some_and(|data| !data.is_empty()) || indices[3].is_none() {
        let (a, b, c, indices) = smart_merge_data_3(a, b, c, [indices[0], indices[1], indices[2]]);
        (a, b, c, None, indices)
    } else if let Some((((a, indices_a), (b, indices_b)), ((c, indices_c), (d, indices_d)))) = a
        .zip(indices[0])
        .zip(b.zip(indices[1]))
        .zip(c.zip(indices[2]).zip(d.zip(indices[3])))
    {
        let d: &[D] = d;
        let (indices, a, b, c, d) =
            merge_data_4(&[indices_a, indices_b, indices_c, indices_d], a, b, c, d);
        let d: Box<[D]> = d;
        (Some(a), Some(b), Some(c), Some(d), Some(indices))
    } else {
        (None, None, None, None, None)
    }
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
    let mut out = std::fs::File::create("target/test_res/susan_unified.obj").unwrap();
    tmf_mesh.write_obj_one(&mut out, &"SUSAN").unwrap();
    assert_eq!(
        tmf_mesh.get_vertex_triangles().unwrap(),
        tmf_mesh.get_normal_triangles().unwrap()
    );
    let mut out = std::fs::File::create("target/test_res/susan_unified.tmf").unwrap();
    tmf_mesh
        .write_tmf_one(&mut out, &TMFPrecisionInfo::default(), "SUSAN")
        .unwrap();
    let mut out = std::fs::File::open("target/test_res/susan_unified.tmf").unwrap();
    let (r_mesh, name) = TMFMesh::read_tmf_one(&mut out).unwrap();
    let mut out = std::fs::File::create("target/test_res/suan_unified_ftmf.obj").unwrap();
    r_mesh.write_obj_one(&mut out, &"SUSAN").unwrap();
}
#[test]
#[cfg(feature = "obj_import")]
fn hand_optimized() {
    use crate::init_test_env;
    use crate::{
        NormalPrecisionMode, TMFMesh, TMFPrecisionInfo, UvPrecisionMode, VertexPrecisionMode,
    };
    init_test_env();
    let mut file = std::fs::File::open("testing/susan.obj").unwrap();
    let mut tmf_mesh = TMFMesh::read_from_obj_one(&mut file).unwrap().0;
    tmf_mesh.verify().unwrap();
    tmf_mesh.reorder_data();
    tmf_mesh.unify_index_data();
    tmf_mesh.verify().unwrap();
    let mut out = std::fs::File::create("target/test_res/susan_ho.tmf").unwrap();
    let tmf_prec = TMFPrecisionInfo {
        normal_precision: NormalPrecisionMode::from_deg_dev(5.0),
        vertex_precision: VertexPrecisionMode(0.5),
        uv_prec: UvPrecisionMode::form_texture_resolution(1024.0, 1.0),
        ..TMFPrecisionInfo::default()
    };
    tmf_mesh
        .write_tmf_one(&mut out, &tmf_prec, "SUSAN")
        .unwrap();
    let mut out = std::fs::File::open("target/test_res/susan_ho.tmf").unwrap();
    let (r_mesh, name) = TMFMesh::read_tmf_one(&mut out).unwrap();
    let mut out = std::fs::File::create("target/test_res/suan_ho_ftmf.obj").unwrap();
    r_mesh.write_obj_one(&mut out, &"SUSAN").unwrap();
}
