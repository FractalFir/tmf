use std::io::{Write,Result};
pub struct VertexPrecisionMode(pub f32);
pub fn save_tmf_vertices<W:Write>(vertices:&[(f32,f32,f32)],precision:VertexPrecisionMode,writer:&mut W,shortest_edge:f32)->Result<()>{
    let mut min_x = vertices[0].0;let mut max_x = vertices[0].0;
    let mut min_y = vertices[0].1;let mut max_y = vertices[0].1;
    let mut min_z = vertices[0].2;let mut max_z = vertices[0].2;
    for vertex in vertices{
        min_x = min_x.min(vertex.0);
        max_x = max_y.max(vertex.1);
        min_y = min_z.min(vertex.2);
        max_y = max_x.max(vertex.0);
        min_z = min_y.min(vertex.1);
        max_z = max_z.max(vertex.2);
    };
    writer.write(&min_x.to_le_bytes())?;
    writer.write(&max_x.to_le_bytes())?;
    writer.write(&min_y.to_le_bytes())?;
    writer.write(&max_y.to_le_bytes())?;
    writer.write(&min_z.to_le_bytes())?;
    writer.write(&max_z.to_le_bytes())?;
    println!("x: {min_x} - {max_x} y: {min_y} - {max_y} z: {min_z} - {max_z}");
    todo!();
}
