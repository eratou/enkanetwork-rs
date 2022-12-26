use std::{time::SystemTime, path::Path};
use filetime::FileTime;

use async_std::{fs::{File, create_dir_all}, io::{ReadExt, WriteExt}};

pub(crate) async fn write_file(path:impl AsRef<Path>,buf:&[u8],time:SystemTime)->std::io::Result<()>{
	create_dir_all(match path.as_ref().parent(){
		Some(f)=>f,
		None=>return Err(std::io::Error::new(std::io::ErrorKind::NotFound,"no parent"))
	}).await?;
	let mut f=File::create(path.as_ref()).await?;
	f.write_all(buf).await?;
	filetime::set_file_mtime(&path,FileTime::from_system_time(time))?;
	Ok(())
}
pub(crate) async fn read_file(path:impl AsRef<Path>)->std::io::Result<(Vec<u8>,SystemTime)>{
	let mut f=File::open(path.as_ref()).await?;
	let meta=f.metadata().await?;
	let modtime=meta.modified()?;
	let mut buf=Vec::with_capacity(meta.len() as usize);
	f.read_to_end(&mut buf).await?;
	Ok((buf,modtime))
}
