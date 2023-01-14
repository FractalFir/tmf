/// Module used to handle reads of data which is not bit aligned(for example, 3 or 17 bits). This is the module that allows for heavy compression used in this format.
pub mod unaligned_rw;
pub mod normals;
use std::io::{Write};
struct FileMetadata{
    author:String,
}
use std::io::Result;
struct TMFMesh{
    metadata:Option<FileMetadata>,
    normals:Option<Box<[(f32,f32,f32)]>>,
    normal_faces:Option<Box<[u32]>>,
}
impl TMFMesh{
    fn write_tmf<W:Write>(&self,w:&mut W)->Result<()>{
        todo!();
    }
}
