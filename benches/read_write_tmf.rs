#[macro_use]
extern crate bencher;

use bencher::Bencher;
use tmf::unaligned_rw::{UnalignedRWMode, UnalignedReader, UnalignedWriter};
fn read_tmf(bench: &mut Bencher) {
    use std::io::Read;
    use tmf::TMFMesh;
    let mut file = std::fs::File::open("testing/susan_bench.tmf").unwrap();
    let mut out = Vec::new();
    file.read_to_end(&mut out);
    bench.iter(|| {
        let r_mesh = TMFMesh::read_tmf_one(&mut (&out as &[u8])).unwrap();
    });
}
fn write_tmf(bench: &mut Bencher) {
    use std::io::Read;
    use tmf::TMFMesh;
    use tmf::TMFPrecisionInfo;
    let mut file = std::fs::File::open("testing/susan_bench.tmf").unwrap();
    let mut out = Vec::new();
    file.read_to_end(&mut out);
    let r_mesh = TMFMesh::read_tmf_one(&mut (&out as &[u8])).unwrap().0;
    let mut prec = TMFPrecisionInfo::default();
    //prec.prune_normals = false;
    bench.iter(|| {
        // clear out
        out.clear();
        r_mesh.write_tmf_one(&mut out, &prec, &"").unwrap();
    });
}
benchmark_group!(benches, read_tmf, write_tmf);
benchmark_main!(benches);
