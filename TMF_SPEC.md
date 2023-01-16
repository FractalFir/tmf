# Tight Model Format File Specification draft version 1.0

## 1. General overview
Data layout: TMF file consists of the file header, containg the magic numbers and version info, and any number of segements, containingd data about model.
Each segment begins with the segment header, containing segment type info - specifing what data segment contins, and segment length in bytes(not inluding the segement header) - used for skipping unknown or unneded segments. After the header, segment contains *length* bytes.
All data in the header is litle-enidinain if not statet otherwise!
## 2. Definitions
### 2.1 Byte-Aligned types
All byte aligned types used in this specification are little-enidian and described using base type *c* and bit number *n* in the following way: *cn*, eg. (u16 - **u**nsignedint of size **16**)
#### 2.1.1 Base Type definition
|type prefix *c*| type definition |
|---------------|-----------------|
| f | litle-enidian float of size *n* |
| u | litle-enidian u of size *n* |
### 2.1 Unaligned Binary Type 
Unaligned binary types can exist only in a Unaligned Binary Array
## 3. Unaligned Binary Arrays 
While UBA itself is byte-aligned, data in ualigned arrays **may not be aligned to byte boundaries**! Data may begin and end in any place inisde a byte in UBA. Next entry in UBA begins in the bit after the bit last entry ends. Empty space at the end of an UBA is padded by 0's to the nearest byte boundary.

## 4 Base file structure:
The file consists of TMF header and any number of segments. All Segment begin with a segment header.
Unknown segments, or segment which are not needed, can **always** be safely ignored, and file can have any combination of segments in any order, with few limitations:
 1. No "orphan* segments - you can't have segments, which refrence data from a segment that does not exist. Eg. Face Vertex Index Segment can't be present without a Vertex Segment, because data it references must be stored in a Vertex Segment, so data it contains will always be invlaid(indices will always be outside array of size 0).
 2. Not all segments can be present more than once in a file. In case of those segments, it is explicitly stated that they can't be present more than once. Generaly, all segments that are referneced by other segments *should* either not be doubled or specify which segment they reference, because order of segments should be treated as always *arbitray*, even if segments usualy occur in certain order.
 3. All segments *must* decode independently. This means that segment *A* can be decoded before or after segment *B* - no segment relies on other segments to be decoded. This does **not** mean that segments can't reference each other - it only means they can't use data from other segments during **decoding**, since order of segments can be *arbitrary*.
### 4.1 TMF Header
A TMF Header is 7 bytes in size and contains:
1. Magic Number -  3 bytes(unsgined ASCII chars) making up the string "TMF", used in file format identification.
2. Major - u16 describing the major version of the format file was saed with.
4. Minor - u16 describing the minor version of the format file was saved with.
### 4.2 Segment Header:
Segment Header contains information necesary to identify segment type, and then skip or process segment. Segment Header always has the same data layout and size, no mater the type of segment. 
1. SegmentType - u16 identifing a particular segement type, for value coresponding to any segment type type look into part of specification of that type. This header contins
2. SegmentDataLength - a u32 specifing, how many bytes this segments data section contains. This **is not** the size of the whole segment(size of SegmentHeader is **not** included here). Only the size of SegmentData is included.
3. SegmentData - array of bytes *SegmentDataLength* in size. Contains all data encoded by this segment, and metadata required to decode it.
## 5.1 Segment Types and how to decode those segments
### 5.1.1 Face Segment Types
All face segment types all have the exact same layout. Intiegers inside them index into the table associated with the type and specify data for each vertex of a triangle.
| SegementType | Type index | Referenced Segment |
|--------------------|------------|--------------------|
| VertexFaceSegement | 2 | VertexSegment |
| NormalFaceSegement | 4 | NormalSegment |
| UvFaceSegment | 6 | UvSegment | 
1. Precision - u8 specifing number of bits used to save each index in face array table
2. FaceCount - u32 specifing number of faces in the model 
3. FaceIndexArray - UBA containg index unaligned unsgined intigters of size precision bits.
### 5.1.2 Vertex Segment
Segment type index of this segment is 1.
Vertices of the model are saved as initgerers representing a fraction of a number 2^save_precision - 1, such that the lowest fraction possible is 0.0 and higest fraction possible is 1.
This vector represents offset form the lower corner of the mesh bounding box scaled by mesh size such that 0.0 is the coordinate of the "lowest" point of the mesh, and 1.0 is the positon of the "highest" point in the mesh. Vertex Segment conisits of:
1. VertexCount - u32 describing how many vertices are in the vertex_array.
2. MinX,MaxX,MinY,MaxY,MinY,MaxZ - f32 each, mesh bounding box used for resizing & reoffseting the mesh during decoding process.
3. PrecX,PrecY,PrecZ - u8 each describing how much precision each component of the saved vector has. 
4. Vertices - UBA containing *VertexCount* vectors of size prec_x+prec_y+prec_z bits. Components lay in memory one after another, not respecting byte boundaries. 
## 5.1.3 Normal Segment: 
Normal Segment index is 3.
		//A lot of Math Magic, to be explained
	TBD
## 5.1.4 UV Segment
UV segment index is 5.
Each component of each uv is saved as an unsigned intieger of size *Precision* bits, and is treated as a fraction of number 2^Precision - 1. 
It consists of:
1. Precision - u8 informing how much precision UV has.
2. Count - u32 informing how many UV's are in this array.
3. UBA containg count 2D vectors with *Count* components of size *Precision*.

