# Tight Model Format
Tight model format is a model format focused on packing model data as tightly as possible, reducing model file sizes. While used model compression is lossy, quality of compression can be easily set, and default settings try to preserve such level of detail, that compressed and not compressed models can't be distinguished by a human eye.
## Intended use case
Tight Model Format is intended to be used as a way of storing compressed 3D game assets. Models can be compressed during build time, reducing size of the game. This means that it is **not** intended as a way to store models during asset creation. The compression algorithm is lossy and reduced details may lead to artefacts when applying modifiers to the model. 
# Compression results
Model used for benchmark is the monkey head ("susan") form 3D modelling software blender subdivided 2 times with smooth shading enabled.
//TODO: compression results performance
## Drawbacks
### WIP
This format 
# How is data compressed and stored inside a TMF file?
Overview of the compression methods and data storage process is described in depth in TMF specification in file TMF_SPEC.md
