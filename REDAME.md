# Tight Model Format
Tight model format is a model format focused on packing model data as tightly as possible, reducing model file sizes. While used model compression is lossy, quality of compression can be easily set, and default settings try to preserve such level of detail, that compressed and not compressed models can't be distinguished by a human eye.
## How are vertex positions compressed?
When model is compressed, a smallest possible cube counting the entire model is found. Next, length of the smallest edge is found, and maximal acceptable compression offset is calculated based on this length(It is set to a configurable percentage of the edge, default 1%). Based on that, size  and position of decimal of a fixed-point number that can describe each po
