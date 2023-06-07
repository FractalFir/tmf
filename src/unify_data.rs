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
        let curr_id = *combos.entry(curr_combo).or_insert_with(||{let id = combo_id;combo_id += 1;id});
        new_indices.push(curr_id as IndexType);
        //println!("curr_combo:{curr_combo:?}");
    }
    for sub_index in 0..DATA_COUNT {
        println!("data_count:{}",indices[sub_index].iter().max().unwrap_or(&0));
    }
    let mut combos = combos.iter().collect::<Vec<_>>();
    combos.sort_by(|a,b|{a.1.cmp(&b.1)});
    let combos:Box<[_]> = combos.iter().map(|(combo,_id)|{combo}).collect();
    println!("combo_count:{},index_count:{}",combos.len(),indices[0].len());
    let mut data_map:Box<[_]> = vec![Vec::with_capacity(len);DATA_COUNT].into();
    for combo in combos.iter(){
        for i in 0..DATA_COUNT{
            data_map[i].push(combo[i]);
        }
    }
    let data_map:Box<[Box<[_]>]> = data_map.into_iter().map(|v|{(*v).clone().into()}).collect();
    (new_indices.into(),data_map.into())
}
pub(crate) fn merge_data<A: Copy, B: Copy>(
    indices: &[&[IndexType]; 2],
    a: &[A],
    b: &[B],
) -> (Box<[IndexType]>, Box<[A]>, Box<[B]>) {
    let (indices, index_map) = unfiy_data_common::<2>(indices);
    let a:Box<[_]> = index_map[0].iter().map(|index|{a[*index as usize]}).collect();
    let b:Box<[_]> = index_map[0].iter().map(|index|{b[*index as usize]}).collect();
    (indices,a,b)
}
#[test]
#[cfg(feature = "obj_import")]
fn read_susan_obj() {
    use crate::init_test_env;
    use crate::{TMFMesh,TMFPrecisionInfo};
    init_test_env();
    let mut file = std::fs::File::open("testing/susan.obj").unwrap();
    let mut tmf_mesh = TMFMesh::read_from_obj_one(&mut file).unwrap().0;
    tmf_mesh.verify().unwrap();
    tmf_mesh.reorder_data();
    tmf_mesh.unify_index_data();
    tmf_mesh.verify().unwrap();
    let mut out = std::fs::File::create("target/test_res/suan_unifed.obj").unwrap();
    tmf_mesh.write_obj_one(&mut out, &"SUSAN").unwrap();
    assert_eq!(tmf_mesh.get_vertex_triangles().unwrap(),tmf_mesh.get_normal_triangles().unwrap());
    let mut out = std::fs::File::create("target/test_res/suan_unifed.tmf").unwrap();
    tmf_mesh.write_tmf_one(&mut out,&TMFPrecisionInfo::default(), "SUSAN").unwrap();
    let mut out = std::fs::File::open("target/test_res/suan_unifed.tmf").unwrap();
    let (r_mesh, name) = TMFMesh::read_tmf_one(&mut out).unwrap();
}
