# What is Tight Model Format
The main goal of the tmf project is to provide a way to save 3D game asset compressed in such a way, that there are no noticeable changes to quality of the model when it is used in a game. This is achieved without sacrificing model read speeds, at cost of increased model write speed.
## How does it work?
While I mark tmf as a "lossy compression format" in a classical meaning of this word it relay does not *compress* anything(at least now). Bulk of the space savings come from storing the model data in diffrent data structures that better reflect the data they store, and removing some data *precision* from the model in such a way, that it's **topology does not change** and difference between more precise original data and less precise saved data is not noticeable by any human, even on close inspection.
# Comparisions
The model sued in test is the blender monkey(Suzzane). TMF files were compressed with default settings.
## File size comparison
| Format | Size |
|--------|------|
| .obj   | 1.3 MB |
| zip(deflate) compressed .obj | 367.7 kB |
| .fbx | 651.0 kB |
| zip(deflate) compressed .fbx | 600.6 kB |
| .tmf |  308.3 kB |
| zip(deflate) compressed .tmf | 308.0 kB |
## Model render comparison
| Uncompresssed .obj | Compressed .tmf file(default settings) |
| ------------------ | ---------------------------------------|
| <img src="docs/original.png"> | <img src="docs/tmf.png"> |
## Math-based savings
Many formats used for saving of 3D models are shockingly wasteful. There is *a lot* of opportunities to reduce file size, even when using lossless compression. For example, many model formats treat surface normal vectors like any other vectors. But they *aren't* like other vectors! They have some special properties, which can be exploited to save them more efficiently. Namely:
1. All components in a normal vector fall into range <-1,1>. This means that values such as 1.3, 123.0, 69.323, or even 6.50e+12 can **never** occur in a normal vector, so saving them using a format which supports those values is wasteful.
2. All normal vectors fulfil conditions x^2 + y^2 + z^2 = 1. This means, that there are a *lot* of vectors that have all their compoents in range <-1,1> but aren't valid surface normals. If saving those invalid values is supported, this means that there is wasted space. 
So, by taking into consideration those properties of normals, they can be saved in such a way that each combination of saved bits correspond to a different normal, wasting no space! 

Analogical approach is taken for each and every element of model data, reducing the size even further,
## Bits vs Bytes based savings.
*TODO: Explaing UBA-s*

