use std::io::{Write,Read,Result};
use crate::unaligned_rw::{UnalignedReader,UnalignedWriter,UnalignedRWMode};
#[derive(Clone,Copy)]
struct NormalPrecisionMode(u8);
const SIGN_PREC:UnalignedRWMode = UnalignedRWMode(1);
fn save_normal<W:Write>(normal:(f32,f32,f32),precision:NormalPrecisionMode,writer:&mut UnalignedWriter<W>)->Result<()>{
    let multiplier = ((1<<precision.0)-1) as f32;
    
    let x = (normal.0.abs()*multiplier) as u64;
    let sx = (normal.0 < 0.0) as u8 as u64;
    let y = (normal.1.abs()*multiplier) as u64;
    let sy = (normal.1 < 0.0) as u8 as u64;
    let sz = (normal.2 < 0.0) as u8 as u64;
    let main_prec = UnalignedRWMode(precision.0);
    writer.write_unaligned(SIGN_PREC,sx)?;
    writer.write_unaligned(main_prec,x)?;
    writer.write_unaligned(SIGN_PREC,sy)?;
    writer.write_unaligned(main_prec,y)?;
    writer.write_unaligned(SIGN_PREC,sz)?;
    println!("WRITE: x:{x},sx:{sx},y:{y},sy:{sy},sz:{sz}");
    Ok(())
}
fn read_normal<R:Read>(precision:NormalPrecisionMode,reader:&mut UnalignedReader<R>)->Result<(f32,f32,f32)>{
    let main_prec = UnalignedRWMode(precision.0);
    let divisor = ((1<<precision.0)-1) as f32;
    
    let sx = if reader.read_unaligned(SIGN_PREC)? != 0{-1.0}else{1.0};
    let x = reader.read_unaligned(main_prec)?;
    let sy = if reader.read_unaligned(SIGN_PREC)? != 0{-1.0}else{1.0};
    let y = reader.read_unaligned(main_prec)?;
    let sz = if reader.read_unaligned(SIGN_PREC)? != 0{-1.0}else{1.0};
    let fx = ((x as f32)/divisor) * sx;
    let fy = ((y as f32)/divisor) * sy;
    let fz = (1.0 - (fx*fx) - (fy*fy)).sqrt() * sz;
    Ok((fx,fy,fz))
}
#[cfg(test)]
mod test_normal{
    use super::*;
    fn dst(a:(f32,f32,f32),b:(f32,f32,f32))->f32{
        let dx = a.0 - b.0;
        let dy = a.1 - b.1;
        let dz = a.2 - b.2;
        return dx*dx+dy*dy+dz*dz;
    }
    fn test_save(normal:(f32,f32,f32)){
        let mut res = Vec::with_capacity(64);
        let precision = NormalPrecisionMode(4);
        {
            let mut writter = UnalignedWriter::new(&mut res);
            save_normal(normal,precision,&mut writter).unwrap();
        }
        let mut reader = UnalignedReader::new(&res as &[u8]);
        let r_normal = read_normal(precision,&mut reader).unwrap();
        assert!(dst(r_normal,normal) < 0.001);
    }
    #[test]
    fn x_axis_rw(){
        test_save((1.0,0.0,0.0));
        test_save((-1.0,0.0,0.0));
    }
    #[test]
    fn y_axis_rw(){
        test_save((0.0,1.0,0.0));
        test_save((0.0,-1.0,0.0));
    }
    #[test]
    fn z_axis_rw(){
        test_save((0.0,0.0,1.0));
        test_save((0.0,0.0,-1.0));
    }
}
