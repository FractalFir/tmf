# **TMF - high compression ratio(up to 89%), blazing fast 3D model format**
# What is TMF?
`tmf` is a model format focused on:
1. Preserving graphical fidelity
2. Achieving high compression ratio
3. Being very fast
4. Giving a very friendly and explicit API, with high quality documentation.
## What is `tmf` best at?
tmf works best when operating on moderately sized 3D models(<100 k triangles), with fairly consistent LOD.
# How good is TMF at achieving its goals?
1. As for the visual quality, one can easily [judge by themselves](#Model-render-comparison).
2. Compression ratio usually falls between 86-89%, depending on quality settings. On more strict settings(preserving exact order of all vertex data) `tmf` has a compression ratio of around ~70%
3. Decode speeds are very high, in some cases outperforming readers of uncompressed formats by a factor of magnitude.
For example, decoding the blender test moneky(Suzanne, file in `tests` directory) subdivided 2 times(15.7k triangles, 8.2k points) takes just **678.18 µs**(*0.67818 ms*)! Thanks to built-in `tokio` integration, decoding may be autmaticaly split between threads, taking decode speeds even further up. Decoding the mesh containing the bust of Nefertiti, a 3D model with around 2 milion triangles, takes **220-240 ms** on a single thread, and only **84 ms** on 8 threads, on a 8 logical core system(4 physical cores). This means that `tmf` is very fast. Please, however, note that `tmf`-s compression algorithm struggles with very large models, so models with millions of triangles receive far less benefits in terms of file size.  
4. TMF API centres mostly around 2 types: `TMFMesh` representing a mesh and all operations that may be done with it, and `TMFPrecisionInfo` specifying quality settings. All of TMFs functions and types are well documented, very often with multiple examples, showing exactly how to use them, greatly improving the ease of use. All operations on a mesh are explicit. 
## Model render comparison
| Uncompressed .obj | Compressed .tmf file(default settings, data reordering allowed) |
| ------------------ | ---------------------------------------|
| <img src="docs/original.png"> | <img src="docs/tmf.png"> |
# Is tmf a right fit for your project?
## When it is not a right fit:
1. You don't care about read speeds at all. Then just use Draco. It is way slower(In my tests around 10-20x), but it is also better at compressing.
2. Your meshes are **very** big(millions of triangles). `tmf` was optimised and tested with much more modest meshes(>150k  triangles). It's compression becomes worse, the more triangles and points you have. It is very still fast, the compression is just not well suited for such tasks.
## When it is a right fit:
1. You need your models to be smaller, but don't want to sacrifice much of the read speed.
2. Your meshes are modestly sized or small (>150k triangles).
3. You only need your meshes to look exacly the same, and are fine with some unnoticable changes.
# How are high compression speeds achieved?
Currently on default settings TMF uses bit-wise operations(bit-shift and  or) to read data, which makes it able to read data at very high speeds. Additionally TMF is thread safe, and has built-in, optional multi-threading, allowing for decoding of many parts of one model at the same time by many cores, increasing speed even further.
# How does it work?
While I mark tmf as a "lossy compression format" in a classical meaning of this word, it really does not *compress* anything (at least for now). The bulk of the space savings come from storing the model data in different data structures that better reflect the data they store, and saving data with exactly precision it needs(e.g. 9 or 23 bit data types). 
# Comparisions
The model used in test is the blender monkey(Suzzane). TMF files were saved with default settings(`TMFPrecisionInfo::default()`).
## File size comparison
| Format | Size |
|--------|------|
| .obj   | 1.3 MB |
| zip(deflate) compressed .obj | 367.7 kB |
| .fbx | 651.0 kB |
| zip(deflate) compressed .fbx | 600.6 kB |
| .gltf | 476.5 kB |
| zip(deflate) compressed .gltf | 302.1 kB |
| .glb |  356.6 kB|
| zip(deflate) compressed .glb| 267.5 kB |
| .tmf |  308.3 kB |
| .tmf with pre-encode optimisations applied | **161.9 kB**| 
| .tmf with pre-encode optimisations and hand-picked quality settings | **142.4 kB** | 
| zip(deflate) compressed .tmf | 307.9 kB |
| zip(deflate) compressed .tmf, with pre-encode optimizations | 160.2 kB |
| zip(deflate) compressed .tmf, with pre-encode optimizations, hand-picked quality settings | 141.0 kB |
| zip(deflate) compressed .tmf, with pre-encode optimizations | 162.8 kB |
| zip(deflate) compressed .tmf, with pre-encode optimizations, hand-picked quality settings | 145.0 kB |
| draco on max compression settings| ~22 kB | 
## TMF vs. Draco.
Draco is noticeably better at compression than TMF. If all you are looking for is reduced file size, then just use Draco. But if you are looking for both high compression and fast reads, tmf can be a vaible alternative.
### A comparison of some pros and cons
*NOTE: when compression ratios/percentages are given, all formats(eg. tmf,draco,fbx) are compared to .obj as uncompressed base*.
| Category | Draco | TMF | 
|----------|-------|-----|
| Compression Ratio | Draco is generally better at compressing data, depending on the compression settings it can be between ~80-98% | TMF can compress your file by around 87.3% | 
| 3D model(Suzanne) read time | 7-10 ms | ~0.6 ms |
| Impact of compression on read time | Read time increases with compression level | For most settings read time **decreases** with compression level |
| 3D model(Suzanne) write time | 10-18 ms | ~7 ms |
| Language | C++ | Rust |
| Official Rust support | None | Native |
| Build Dependencies | C++ compiler, cmake, make | only standard rust tollchain |
| Using in rust project | Requires manual linking | installs and links automatically using cargo |
## What can lead to compression of a particular mesh being less efficient?
Greatly varying LOD: The save system dynamically adjusts to the LOD of the mesh. For example, a low-poly castle mesh may be saved with precision of 10 cm and a strawberry model may be saved with 1 mm precision. Saving those two object in one *mesh*(not file!) will force the castle mesh to be saved with higher precision, wasting space. Because most meshes will naturally have a consistent LOD, and meshes that don't would almost always lead to issues elsewhere, this problem is rarely encountered.
# Examples
## Mesh loading
Loading one mesh
```rust
use tmf::TMFMesh;
use std::fs::File;
let input = File::open("suzanne.tmf").expect("Could not open .tmf file!");
let (mesh,name) = TMFMesh::read_tmf_one(&mut input).expect("Could not read TMF file!");
// Geting mesh data
let vertices = mesh.get_vertices().expect("No vertices!");
let vertex_triangles = mesh.get_vertex_triangles().expect("No vertiex triangle array!");
let normals = mesh.get_normals().expect("No normals!");
let normal_triangles = mesh.get_normal_triangles().expect("No normal triangle array!");
let uvs = mesh.get_uvs().expect("No uvs!");
let uv_triangles = mesh.get_uv_triangles().expect("No uv triangle array!");
// Can provide arrays laid out like OpenGL buffers for ease of use when developing games!
let buff_vert_array = mesh.get_vertex_buffer();
let buff_norm_array = mesh.get_normal_buffer();
let buff_uv_array = mesh.get_uv_buffer();
```
Loading multiple meshes
```rust
use tmf::TMFMesh;
use std::fs::File;
let input = File::open("suzanne.tmf").expect("Could not open .tmf file!");
let meshes = TMFMesh::read_tmf_one(&mut input).expect("Could not open TMF file!");
for (mesh,name) in meshes{
    do_something(mesh,name);
}
```
## Mesh Saving
Saving one mesh
```rust
use tmf::TMFMesh;
use std::fs::File;
let output = File::create("suzanne.tmf").expect("Could not create output file!");
let settings = TMFPrecisionInfo::default();

// Change TMF mesh to have better laid out data. This can save significant ammounts of space.
mesh.unify_index_data();

mesh.write_tmf_one(&mut output,&settings,name).expect("Could not save TMF mesh!");
```
Saving multiple meshes
```rust
use tmf::TMFMesh;
use std::fs::File;
let output = File::open("suzanne.tmf").expect("Could not create .tmf file!");
TMFMesh::write_tmf(meshes,&mut input,&settings).expect("Could not write TMF mesh!");
```
# Features
# 0.1 (Current version)
- [X] Exporting .obj
- [X] Importing .obj
- [X] Importing not-triangulated .obj models - **experimental**, supports only convex polygons
- [X] Writing/Reading .tmf files
- [X] Point Positions
- [X] Point Normals
- [X] Point Uv coordinates
- [X] Mesh triangles
- [X] Point clouds
- [X] Multiple meshes in one file
- [X] Fully customizable save precision settings
- [X] Full documentation
- [X] Examples for each function in the crate
- [X] Tangent data
- [X] Custom mesh data
- [X] Support for RGBA vertex colors(grayscale support using a float attribute), and float/integer vertex attributes.
# Planed Features
- [ ] Vertex groups 
- [ ] Materials *some initial work already done*
# More in-depth explanation of compression
## Math-based savings
Many formats used for saving of 3D models are shockingly wasteful. There are *a lot* of opportunities to reduce file size, even when using lossless compression. For example, many model formats treat surface normal vectors like any other vectors. But they *aren't* like other vectors! They have some special properties, which can be exploited to save them more efficiently. Namely:
1. All components in a normal vector fall into range <-1,1>. This means that values such as 1.3, 123.0, 69.323, or even 6.50e+12 can **never** occur in a normal vector, so saving them using a format which supports those values is wasteful.
2. All normal vectors fulfil conditions x^2 + y^2 + z^2 = 1. This means, that there are a *lot* of vectors that have all their components in range <-1,1> but aren't valid surface normals. If saving those invalid values is supported, this means that there is wasted space. 
So, by taking into consideration those properties of normals, they can be saved in such a way that each combination of saved bits correspond to a different normal, wasting no space! 

An analogical approach is taken for each and every element of model data, reducing the size even further.
## Bits vs Bytes based savings.
A disadvantage of using byte-aligned data types is lack of granularity of precision when saving data. A good example of this may be a UV coordinate that should represent a point on a 1024 pixel texture, with precision of .25 pixels. Doing some quick back of the napkin maths, it can be determined that a precision of log2(1024/.25) = log2(4096) = 12 bits is required. But only available data types are either too small (u8) or way too big(u16, 25% of disk space would go to waste!). The solution is forgoing byte alignment. It comes with a slight performance penalty of having to do bit shifts, and inability to use pre-built compression algorithms (they assume byte-alignment), but come with huge advantage of using data types just wide enough to save what is needed and not any wider. 
Data is laid out like that in what I call an UBA (Unaligned Binary Array). Data in an UBA consists of a series of data with any binary size, where consecutive data may cross byte boundaries, start or end at any point in a byte, and there is no padding. The size of elements is usually specified before the UBA itself. For some widths, like 9 bits, savings coming from using UBA's can reach as much as 44%!
# Specification
Along with this project comes a slightly more in-depth technical specification
in `TMF_SPEC.md` *NOTE: this specification is outdated, as it describes version `0.1`(an updated version is planed)*. While not fully finished, it can still prove to useful for anyone who is interested in understanding the project better(If you have any questions feel free to ask me). 
