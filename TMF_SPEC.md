# Tight Model Format Specification version 0.1
*this is a draft and WIP*
## 1. File structure
### 1.1 TMF Header
TMF Header consist of following data, laid out without padding.
1. Magic - 3 bytes ([0x54,0x4d,0x46]) creating the string "TMF" used for identification
2. TMF\_MAJOR:little-enidian u16 - major version of format used to save file
3. TMF\_MINOR:little-enidian u16 - major version of format used to save file
4. MIN\_TMF\_MAJOR:little-enidian u16 - major version of format reader required to read this file
4. MIN\_TMF\_MINOR:little-enidian u16 - minor version of format reader required to read this file
5. mesh_count: little-enidian u32 - count of meshes inside this file. 
After TMF Header a *mesh_count* meshes will follow, each beginning with the Mesh Header
## 1.2 Mesh Header
Each mesh header begins with mesh name, an UTF-8 string saved as:
1. str_len: little-enidian u16 - describing name length
2. str_bytes: *str\_len* bytes  - making up the name UTF-8 encoded string(not null terminated!)
After the name string there is one more field:
3. seg_count:little-enidian u16 - specifying how many data segment this mesh has
After the mesh header there is *seg_count* segment.
## 1.3 Segment Header
Segment header consists of:
1. seg_type:little-enidian u16 - describing segment type, for specific values look at [Segment Types](#Segment Types).
2. data_length:little-enidian u64 - describing the length of data in the segment(excluding the header)
3. compresion_type:u8 - Describes used compression, for more info look into [CompressionTypes](#Compression Types)
4. data: *data_length* bytes - the internal data of the segment, for what it is exactly for each segment - look into [Segment Types](#Segment Types).
# 2 Segments Types
| value | meaning |
|-------|---------|
| 0 | Invalid | 
| 1 | VertexSegment |
| 2 | VertexTriangleSegment|
| 3 | NormalSegment |
| 4 | NormalTriangleSegment |
| 5 | UvSegment | 
| 6 | UvTriangleSegment |
| 7 | MaterialInfo |
| 8 | Materialtriangles |
## 2.1 Invalid
Segment is invalid, and should be ignored.
## 2.2 Vertex Segment 
This segment stores an array of points and consists of the following data:
1. vertex\_count: little\_enidian u64 - describes amount of vertices ths segment contains.
### The bounding box of points inside this segment
2. min\_x: little\_enidian f64 - minimal x position of any point inside this segment.
3. max\_x: little\_enidian f64 - maximal x position of any point inside this segment.
4. min\_y: little\_enidian f64 - minimal y position of any point inside this segment.
5. max\_y: little\_enidian f64 - maximal y position of any point inside this segment.
6. min\_z: little\_enidian f64 - minimal z position of any point inside this segment.
7. max\_z: little\_enidian f64 - maximal z position of any point inside this segment.
### Save precision of points
8.  prec_x: u8 - describes how many bits the x component of point has
9.  prec_y: u8 - describes how many bits the y component of point has
10. prec_z: u8 - describes how many bits the z component of point has
### Unaligned Binary Array describing points
Each vector inside of this UBA has 3 components, prec\_x, prec\_y and prec\_z bits in size respectively. Each component is saved as a number between 0 and 2^component_precision - 1. So it should be divide by this max number to get a number between 0 and 1. This number specifies where inside of the bounding box the point shall lay, where 0 is the minimal value of this cordiante and 1 is the maximal one.
## 2.3 Vertex, Normal and UV Triangle segments.
Each one of those segments stores an array of indices into the Vertex, Normal and UV Segment respectively. Those indices then create triangles.
The data layout of those segments is: 
1. precision: u8 - describes the size of index in bits
2. count: little-enidian u64 describing the amount of indices.
### UBA containing indices 
Each index is precision bits in size.
## 2.4 Normal Segment
*TODO*
## 2.5 UV Segment
*TODO*
# 3. Compression Types
| value | meaning |
|--|--|
| 0 | No compression |
| 1 | Segment Omitted | 
| 2 | Unaligned LZZ (not implemented, reserved) |
# 3.1 No compression 
Uncompressed, just raw bytes.
## 3.2 Segment Omitted 
Segment is omitted. This *does not* mean that mesh does not have this data, ti only means that the data can be derived from other mesh data.
Only segment that can be currently omitted is `NormalTriangleSegment`
### 3.2.1 NormalTriangleSegment
If normal triangle segment is omitted, it should be assumed that it has the same amount of indices as the normal segment has normal vectors, and that each index is equal to its position. 
Example:
\[[0,1,2,3,4,5,6,7,8,9,10,...]\]
# 4 UBA-s
Binary data in UBA-s does not align to byte boundaries and may cross them. There is no padding between data.
For example a u3 and u5 may be within the same byte. A u12 crosses a byte boundary, but the next u12 may not start at the next byte, but inside the same byte the last data ended in.
