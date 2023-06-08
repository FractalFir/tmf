use criterion::{black_box, criterion_group, criterion_main, Criterion};
fn read_tmf(c: &mut Criterion) {
    use std::io::Read;
    use tmf::TMFMesh;
    let mut file = std::fs::File::open("target/test_res/susan.tmf").unwrap();
    let mut out = Vec::new();
    file.read_to_end(&mut out);
    c.bench_function("read susan.tmf", |b| {
        b.iter(|| {
            let r_mesh = TMFMesh::read_tmf_one(&mut (&out as &[u8])).unwrap();
            black_box(r_mesh);
        })
    });
}
fn read_unified_tmf(c: &mut Criterion) {
    use std::io::Read;
    use tmf::TMFMesh;
    let mut file = std::fs::File::open("target/test_res/susan_unified.tmf").unwrap();
    let mut out = Vec::new();
    file.read_to_end(&mut out);
    c.bench_function("read unified susan.tmf", |b| {
        b.iter(|| {
            let r_mesh = TMFMesh::read_tmf_one(&mut (&out as &[u8])).unwrap();
            black_box(r_mesh);
        })
    });
}
fn read_nefreti_tmf(c: &mut Criterion) {
    use std::io::Read;
    use tmf::TMFMesh;
    let mut file = match std::fs::File::open("target/test_res/Nefertiti.tmf") {
        Ok(file) => file,
        Err(_) => return,
    };
    let mut out = Vec::new();
    file.read_to_end(&mut out);
    c.bench_function("read nefretiti.tmf", |b| {
        b.iter(|| {
            let r_mesh = TMFMesh::read_tmf_one(&mut (&out as &[u8])).unwrap();
            black_box(r_mesh);
        })
    });
}
fn write_tmf(c: &mut Criterion) {
    use std::io::Read;
    use tmf::TMFMesh;
    use tmf::TMFPrecisionInfo;
    let mut file = std::fs::File::open("testing/susan.tmf").unwrap();
    let mut out = Vec::new();
    file.read_to_end(&mut out);
    let r_mesh = TMFMesh::read_tmf_one(&mut (&out as &[u8])).unwrap().0;
    let mut prec = TMFPrecisionInfo::default();
    c.bench_function("write susan.tmf", |b| {
        b.iter(|| {
            out.clear();
            r_mesh.write_tmf_one(&mut out, &prec, "").unwrap();
            black_box(&mut out);
        })
    });
}
criterion_group!(
    benches,
    read_tmf,
    read_unified_tmf,
    read_nefreti_tmf,
    write_tmf
);
criterion_main!(benches);
