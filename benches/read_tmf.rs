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
        let r_mesh = TMFMesh::read_tmf(&mut (&out as &[u8])).unwrap();
    });
}
benchmark_group!(benches, read_tmf);
benchmark_main!(benches);
