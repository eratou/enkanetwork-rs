use std::{time::SystemTime, sync::Arc, collections::HashMap};

use async_std::{sync::RwLock, path::{PathBuf, Path}};

#[derive(Debug)]
pub struct MemoryCache{
	disk_dir:Option<String>,
	limit_files:usize,
	limit_size:usize,
	cache:RwLock<PriorityMap>
}
#[derive(Debug)]
struct PriorityMap{
	map:HashMap<String,(Arc<Vec<u8>>,SystemTime)>,
	priority:Vec<String>,
}
impl PriorityMap{
	fn new()->Self{
		Self{
			map:HashMap::new(),
			priority: vec![]
		}
	}
}
impl MemoryCache{
	pub fn from(disk_dir:Option<String>,limit_files:usize,limit_size:usize)->Self{
		Self{
			disk_dir,
			cache:RwLock::new(PriorityMap::new()),
			limit_files,
			limit_size,
		}
	}
	pub fn new(disk_dir:String)->std::io::Result<Self>{
		std::fs::create_dir_all(&disk_dir)?;
		Ok(Self::from(Some(disk_dir),
			32,
			2*1024*1024,
		))
	}
	pub async fn set(&self,local_path:impl AsRef<str>,buf:impl AsRef<[u8]>,time:SystemTime)->std::io::Result<()>{
		if let Some(disk_dir)=&self.disk_dir{
			let mut path=PathBuf::from(disk_dir);
			path.push(Path::new(local_path.as_ref()));
			crate::io::write_file(path,buf.as_ref(),time).await?;
		}
		if self.limit_size>buf.as_ref().len(){
			let mut lock=self.cache.write().await;
			if lock.map.contains_key(local_path.as_ref()){
				return Ok(());
			}
			let data=buf.as_ref().to_vec();
			let lock=&mut *lock;
			self.insert(lock,local_path.as_ref().to_owned(), data, time).await;
		}
		Ok(())
	}
	pub async fn get(&self,local_path:impl AsRef<str>)->std::io::Result<(Arc<Vec<u8>>,SystemTime)>{
		let lock=self.cache.read().await;
		let map=&(*lock).map;
		let cache=map.get(local_path.as_ref());
		match cache {
			Some((v,t))=>{
				//println!("from_memory_cache(read_lock) {}",local_path.as_ref());
				Ok((v.clone(),*t))
			},
			None=>{
				std::mem::drop(cache);
				std::mem::drop(map);
				std::mem::drop(lock);
				match &self.disk_dir{
					Some(disk_dir)=>{
						let mut lock=self.cache.write().await;
						let lock=&mut *lock;
						if let Some((v,t))=lock.map.get(local_path.as_ref()){
							//println!("from_memory_cache(write_lock) {}",local_path.as_ref());
							return Ok((v.clone(),*t));
						}
						let mut path=PathBuf::from(disk_dir);
						path.push(Path::new(local_path.as_ref()));
						let disk_cache=crate::io::read_file(path).await?;
						Ok(self.insert(lock,local_path.as_ref().to_owned(),disk_cache.0,disk_cache.1).await)
					},
					None=>Err(std::io::Error::new(std::io::ErrorKind::NotFound,"no disk cache"))
				}
			}
		}
	}
	async fn insert(&self,lock:&mut PriorityMap,key:String,data:Vec<u8>,time:SystemTime)->(Arc<Vec<u8>>,SystemTime){
		let data=Arc::new(data);
		if lock.map.len()>self.limit_files{
			if let Some(k)=lock.priority.get(0){
				lock.map.remove(k);
			}
		}
		if self.limit_files>0{
			if let Some(f)=lock.priority.iter().position(|x|*x==key){
				let value=lock.priority.remove(f);
				lock.priority.push(value);
			}
			lock.map.insert(key,(data.clone(),time));
		}
		(data,time)
	}
	///unimplemented
	pub async fn set_expire(&self,_local_path:impl AsRef<str>,_time:SystemTime){
		
	}
}
