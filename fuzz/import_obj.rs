#![no_main]

use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    use std::borrow::Borrow;
   fn test<R:std::io::Read>(mut src:R){
    let meshes = tmf::TMFMesh::read_from_obj(&mut src);
}
    test(data);   
});
