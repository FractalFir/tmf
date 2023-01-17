use std::io::{Write,Read,Result};
use crate::unaligned_rw::{UnalignedReader,UnalignedWriter,UnalignedRWMode};
#[derive(Clone,Copy)]
pub struct NormalPrecisionMode(u8);
impl NormalPrecisionMode{
    pub fn from_deg_dev(deg:f32)->Self{
        let prec = (90.0/deg).log2().ceil() as u8;
        Self(prec)
    }
}
pub const NORM_PREC_LOW:NormalPrecisionMode = NormalPrecisionMode(7);
pub const NORM_PREC_MID:NormalPrecisionMode = NormalPrecisionMode(10);
pub const NORM_PREC_HIGH:NormalPrecisionMode = NormalPrecisionMode(13);
const SIGN_PREC:UnalignedRWMode = UnalignedRWMode::precision_bits(1);
// https://www.gamedev.net/forums/topic/621589-extremely-fast-sin-approximation/
type fprec = f64;
const fPI:fprec = std::f64::consts::PI;
fn fsin(mut x:fprec)->fprec{
	let mut z = (x*0.3183098861837907) + 6755399441055744.0;
	let k:i32 = unsafe{*(&z as *const _ as *const _)};
	z = (k as fprec) * fPI;
	x -= z;
	let y = x*x;
	let z = (0.0073524681968701*y - 0.1652891139701474)*y +  0.9996919862959676;
	x *= z;
	let mut k = k & 1;
	k += k;
	let z = (k as fprec) * x;
	return x - z;
}
pub(crate) fn magnitude(i:(f32,f32,f32))->f32{
    let xx = i.0 * i.0;
    let yy = i.1 * i.1;
    let zz = i.2 * i.2;
    (xx + yy + zz).sqrt()
}
fn normalize(i:(f32,f32,f32))->(f32,f32,f32){
    let xx = i.0 * i.0;
    let yy = i.1 * i.1;
    let zz = i.2 * i.2;
    let mag = (xx + yy + zz).sqrt();
    
    (i.0/mag,i.1/mag,i.2/mag)
}
use std::f32::consts::PI as PI;
fn save_normal<W:Write>(normal:(f32,f32,f32),precision:NormalPrecisionMode,writer:&mut UnalignedWriter<W>)->Result<()>{
    let multiplier = ((1<<precision.0)-1) as f32;
    //Calculate asine
    let xy = (normal.0.abs(),normal.1.abs());
    let xy_mag = (xy.0*xy.0 + xy.1*xy.1).sqrt();
    let xy = (xy.0/xy_mag,xy.1/xy_mag);
    let asine = xy.0.asin();

    let asine = asine/(PI/2.0);
    //
    let asine = (asine*multiplier)      as u64;
    let z = (normal.2.abs()*multiplier) as u64;
    let sx = (normal.0 < 0.0) as u8     as u64;
    let sy = (normal.1 < 0.0) as u8     as u64;
    let sz = (normal.2 < 0.0) as u8     as u64;
    let main_prec = UnalignedRWMode::precision_bits(precision.0);
    
    writer.write_unaligned(SIGN_PREC,sx)?;
    writer.write_unaligned(SIGN_PREC,sy)?;
    writer.write_unaligned(SIGN_PREC,sz)?;
    writer.write_unaligned(main_prec,asine)?;
    writer.write_unaligned(main_prec,z)?;
    
    Ok(())
}
fn read_normal<R:Read>(precision:NormalPrecisionMode,reader:&mut UnalignedReader<R>)->Result<(f32,f32,f32)>{
    let main_prec = UnalignedRWMode::precision_bits(precision.0);
    let divisor = ((1<<precision.0)-1) as f32;
    // Get signs of x y z component 
    let sx = reader.read_unaligned(SIGN_PREC)? != 0;
    let sy = reader.read_unaligned(SIGN_PREC)? != 0;
    let sz = reader.read_unaligned(SIGN_PREC)? != 0;
    // Read raw asine
    let asine = (reader.read_unaligned(main_prec)? as f32)/divisor;
    //Convert asine
    let asine = asine * (PI/2.0);
    //Read xyz component
    let z = (reader.read_unaligned(main_prec)? as f32)/divisor;
	#[cfg(feature = "fast_trig")]
    let x = fsin(asine as fprec) as f32; 
	#[cfg(not(feature = "fast_trig"))]
    let x = asine.sin();
   	
	let y = (1.0 - x*x).sqrt();
    // Calculate XY magnitude
    let xy_mag = (1.0 - z*z).sqrt();
    // Adjust x an y
    let y = y * xy_mag;
    let x = x * xy_mag;
    // Set signs
	let x = if sx {-x}else{x};
	let y = if sy {-y}else{y};
	let z = if sz {-z}else{z};
    let res = (x,y,z);
    Ok(res)
}
pub (crate) fn save_normal_array<W:Write>(normals:&[(f32,f32,f32)],writer:&mut W,precision:NormalPrecisionMode)->Result<()>{
    let count = (normals.len() as u32).to_le_bytes();
    writer.write_all(&count)?;
    writer.write_all(&[precision.0])?;
    let mut writer = UnalignedWriter::new(writer);
    for normal in normals{
         save_normal(*normal,precision,&mut writer)?;
    }
    writer.flush()?;
    Ok(())
}
pub (crate) fn read_normal_array<R:Read>(reader:&mut R)->Result<Box<[(f32,f32,f32)]>>{
    let count = {
        let mut tmp:[u8;4] = [0;4];
        reader.read_exact(&mut tmp)?;
        u32::from_le_bytes(tmp)
    } as usize;
    let precision = NormalPrecisionMode({
        let mut tmp:[u8;1] = [0;1];
        reader.read_exact(&mut tmp)?;
        tmp[0]
    });
    let mut reader = UnalignedReader::new(reader);
    let mut normals = Vec::with_capacity(count);
    for _ in 0..count{
        let normal = read_normal(precision,&mut reader)?;
        normals.push(normal);
    }
    Ok(normals.into())
}
pub (crate) fn save_normal_face_array<W:Write>(faces:&[u32],normal_count:u32,writer:&mut W)->std::io::Result<()>{
    let face_count = (faces.len() as u32).to_le_bytes();
    let face_precision = (normal_count as f64).log2().ceil() as u8;
    writer.write_all(&face_count)?;
    writer.write_all(&[face_precision])?;
    let face_precision = UnalignedRWMode::precision_bits(face_precision);
    let mut writer = UnalignedWriter::new(writer);
    for index in faces{
        writer.write_unaligned(face_precision,*index as u64).unwrap();
    }
    writer.flush()?;
    Ok(())
}
#[cfg(test)]
mod test_normal{
    use super::*;
    fn dot(a:(f32,f32,f32),b:(f32,f32,f32))->f32{
        a.0*b.0 + a.1*b.1 + a.2*b.2
    }
    fn test_save(normal:(f32,f32,f32)){
        let mut res = Vec::with_capacity(64);
        let precision = NormalPrecisionMode(14);
        {
            let mut writter = UnalignedWriter::new(&mut res);
            save_normal(normal,precision,&mut writter).unwrap();
        }
        let mut reader = UnalignedReader::new(&res as &[u8]);
        let r_normal = read_normal(precision,&mut reader).unwrap();
        let n_dot = (1.0 - dot(r_normal,normal))*180.0;
        assert!(n_dot < 0.01,"expected:{normal:?} != read:{r_normal:?} angle:{n_dot}");
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
    #[test]
    fn random_axis_rw(){
        use rand::{Rng,thread_rng};
        let mut rng = thread_rng();
        for _ in 0..100000{
            let norm = (rng.gen::<f32>()*2.0 - 1.0, rng.gen::<f32>()*2.0 - 1.0, rng.gen::<f32>()*2.0 - 1.0);
            let norm = normalize(norm);
            test_save(norm);
        }
    }
    #[test]
    fn rw_normal_array(){
        use rand::{Rng,thread_rng};
        let mut rng = thread_rng();
        let count = ((rng.gen::<u32>()%0x800)+0x800) as usize;
        let mut res = Vec::with_capacity(count);
        let mut normals = Vec::with_capacity(count);
        for _ in 0..count{
            let norm = (rng.gen::<f32>()*2.0 - 1.0, rng.gen::<f32>()*2.0 - 1.0, rng.gen::<f32>()*2.0 - 1.0);
            let norm = normalize(norm);
            normals.push(norm);
        }
        save_normal_array(&normals,&mut res,NORM_PREC_HIGH).unwrap();
        let r_normals = read_normal_array(&mut(&res as &[u8])).unwrap();
        for i in 0..count{
            let r_normal = r_normals[i];
            let normal = normals[i];
            let n_dot = (1.0 - dot(r_normal,normal))*180.0;
            assert!(n_dot < 0.1,"expected:{normal:?} != read:{r_normal:?} angle:{n_dot}");
        }
    }
	#[test] 
	fn test_fast_sin(){
		for i in 1..100000{
			let x:fprec = (100000.0/(i as fprec))*std::f64::consts::PI;
			let sin = x.sin();
			let fsin = fsin(x);
			let dt = sin - fsin ;
			assert!(dt < 0.000333,"{x}:{sin} - {fsin} = {dt}");
		}
	}
    #[test]
    fn rw_normal_face_array(){
        use rand::{Rng,thread_rng};
        let mut rng = thread_rng();
        let normal_count = ((rng.gen::<u32>()%0x800)+0x800) as u32;
        let face_count = ((rng.gen::<u32>()%0x800)+0x800) as usize;
        let mut faces = Vec::with_capacity(face_count*3);
        for _ in 0..face_count*3{
            let index = rng.gen::<u32>()%normal_count;
            faces.push(index);
        }
        let mut res = Vec::with_capacity(face_count);
        save_normal_face_array(&faces,normal_count as u32,&mut res).unwrap()
    }
}
