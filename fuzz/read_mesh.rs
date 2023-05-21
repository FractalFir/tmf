#![no_main]

use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    use std::borrow::Borrow;
   fn test<R:std::io::Read>(mut src:R){
    let meshes = tmf::TMFMesh::read_tmf(&mut src);
}
    let mut test_data:Vec<u8> = vec![0x54,0x4D,0x46, 0x0,0x0, 0x1,0x0, 0x0,0x0, 0x0,0x0];
    test_data.extend(data);
    let data:&[u8] = &test_data;
    test(data);   
});
