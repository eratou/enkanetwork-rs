use std::{time::SystemTime, sync::Arc};

use async_std::path::{PathBuf, Path};
use redis::{ RedisError, AsyncCommands};

#[derive(Clone,Debug)]
pub struct MemoryCache{
	disk_dir:Option<String>,
	client:redis::Client,
}
impl MemoryCache{
	pub async fn from(disk_dir:Option<String>,client:redis::Client)->Result<Self,RedisError>{
		Ok(Self{
			disk_dir,
			client,
		})
	}
	#[deprecated]
	pub fn new(disk_dir:String)->std::io::Result<Self>{
		let client=match redis::Client::open("redis://127.0.0.1/"){
			Ok(c)=>c,
			Err(e)=>{
				return Err(std::io::Error::new(std::io::ErrorKind::Other,e.to_string()));
			}
		};
		let disk=match std::fs::create_dir_all(&disk_dir){
			Ok(_)=>Some(disk_dir),
			Err(_)=>None,
		};
		let rt=tokio::runtime::Builder::new_current_thread().enable_all().build()?;
		match rt.block_on(Self::from(disk,client)){
			Ok(s)=>Ok(s),
			Err(e)=>Err(std::io::Error::new(std::io::ErrorKind::Other,e.to_string()))
		}
	}
	pub async fn set(&self,local_path:impl AsRef<str>,buf:impl AsRef<[u8]>,time:SystemTime)->std::io::Result<()>{
		let job=self.insert(local_path.as_ref(),buf.as_ref(),time.clone());
		if let Some(disk_dir)=&self.disk_dir{
			let mut path=PathBuf::from(disk_dir);
			path.push(Path::new(local_path.as_ref()));
			let (fw,nw)=futures::join!(crate::io::write_file(path,buf.as_ref(),time),job);
			nw?;
			fw?;
		}else{
			let _=job.await?;
		}
		Ok(())
	}
	pub async fn get(&self,local_path:impl AsRef<str>)->std::io::Result<(Arc<Vec<u8>>,SystemTime)>{
		let mut map=match self.client.get_async_connection().await{
			Ok(c)=>c,
			Err(e)=>{
				return Err(std::io::Error::new(std::io::ErrorKind::Other,e.to_string()));
			}
		};
		let cache:Result<Vec<u8>,RedisError>=map.get(local_path.as_ref()).await;
		match cache {
			Ok(mut v)=>{
				if v.len()<8{
					return self.from_disk(local_path.as_ref()).await;
				}
				//println!("data {} bytes",v.len());
				let meta:Vec<u8>=v.drain(0..8).collect();
				let time=i64::from_be_bytes([meta[0],meta[1],meta[2],meta[3],meta[4],meta[5],meta[6],meta[7]]);
				let time=chrono::DateTime::<chrono::Utc>::from_utc(chrono::NaiveDateTime::from_timestamp_millis(time).unwrap(),chrono::Utc);
				//println!("from_redis_cache {}",local_path.as_ref());
				Ok((Arc::new(v),time.into()))
			},
			Err(_)=>{
				std::mem::drop(cache);
				std::mem::drop(map);
				self.from_disk(local_path.as_ref()).await
			}
		}
	}
	async fn from_disk(&self,local_path:&str)->std::io::Result<(Arc<Vec<u8>>,SystemTime)>{
		match &self.disk_dir{
			Some(disk_dir)=>{
				let mut path=PathBuf::from(disk_dir);
				path.push(Path::new(local_path));
				let disk_cache=crate::io::read_file(path).await?;
				let _=self.insert(local_path,&disk_cache.0,disk_cache.1).await;
				//println!("from_disk_cache {}",local_path);
				Ok((Arc::new(disk_cache.0),disk_cache.1))
			},
			None=>Err(std::io::Error::new(std::io::ErrorKind::NotFound,"no disk cache"))
		}
	}
	async fn insert(&self,local_path:&str,buf:&[u8],time:SystemTime)->std::io::Result<()>{
		let mut v=Vec::with_capacity(buf.len()+8);
		let utc:chrono::DateTime<chrono::Utc>=time.into();
		v.extend_from_slice(&utc.timestamp_millis().to_be_bytes());
		v.extend_from_slice(buf);
		//println!("write_redis_cache {}",local_path);
		let mut con=match self.client.get_async_connection().await{
			Ok(c)=>c,
			Err(e)=>{
				return Err(std::io::Error::new(std::io::ErrorKind::Other,e.to_string()));
			}
		};
		let r:redis::RedisResult<()>=con.set(local_path,v).await;
		if let Err(e)=r{
			eprintln!("write_redis_error {}",e.to_string());
			Err(std::io::Error::new(std::io::ErrorKind::Other,e.to_string()))
		}else{
			Ok(())
		}
	}
	pub async fn set_expire(&self,local_path:impl AsRef<str>,time:SystemTime){
		let mut con=match self.client.get_async_connection().await{
			Ok(c)=>c,
			Err(_)=>{
				return;
			}
		};
		let utc:chrono::DateTime<chrono::Utc>=time.into();
		let _:Result<(),RedisError>=con.pexpire_at(local_path.as_ref(),utc.timestamp_millis() as usize).await;
	}
}
