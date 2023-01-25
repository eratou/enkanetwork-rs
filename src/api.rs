use std::{time::SystemTime, sync::Arc};
use image::DynamicImage;
use reqwest::{header::HeaderMap, Client, ClientBuilder};

use crate::{RawUserData, UserData,store::{self, StoreValue}, FightPropLocale, MemoryCache};

#[derive(Clone)]
pub struct EnkaNetwork{
	pub(crate)assets_cache:Arc<MemoryCache>,
	user_cache:Arc<MemoryCache>,
	client:Option<Client>,
	header:Option<Arc<HeaderMap>>,
	store:Option<Arc<StoreValue>>,
}
impl EnkaNetwork{
	#[cfg(not(target_arch = "wasm32"))]
	fn client_builder()->ClientBuilder{
		Client::builder().timeout(std::time::Duration::from_secs(30)).user_agent(crate::USER_AGENT)
	}
	#[cfg(target_arch = "wasm32")]
	fn client_builder()->ClientBuilder{
		Client::builder()
	}
	pub fn new()->std::io::Result<Self>{
		let client=Self::client_builder();
		let client=client.build().ok();
		let assets_cache=MemoryCache::new(String::from("./cache/assets/"))?;
		let user_cache=MemoryCache::new(String::from("./cache/u/"))?;
		let mut api=Self::from(client,assets_cache,user_cache);
		if let Ok(rt)=tokio::runtime::Builder::new_current_thread().enable_all().build(){
			api.set_store(rt.block_on(api.store()).ok());
		}
		Ok(api)
	}
	pub fn set_header(&mut self,header:Option<HeaderMap>){
		match header{
			Some(header)=>{
				self.header=Some(Arc::new(header));
			},
			None=>self.store=None
		}
	}
	pub fn set_store(&mut self,store:Option<StoreValue>){
		match store{
			Some(store)=>{
				self.store=Some(Arc::new(store));
			},
			None=>self.store=None
		}
	}
	pub fn get_store(&self)->Result<&StoreValue,String>{
		match &self.store{
			Some(v)=>Ok(v),
			None=>Err(String::from("no store"))
		}
	}
	pub async fn store(&self)->Result<StoreValue,String>{
		store::store(&self).await
	}
	pub fn from(client:Option<Client>,assets_cache:MemoryCache,user_cache:MemoryCache)->Self{
		Self{
			assets_cache:Arc::new(assets_cache),
			user_cache:Arc::new(user_cache),
			client,
			header:None,
			store:None,
		}
	}
	pub async fn reload(&self,data:&UserData)->Result<Option<UserData>,String>{
		let lastupdate=SystemTime::now();
		if data.reload_time()>=lastupdate{
			Ok(None)
		}else{
			let raw=self.fetch_user(data.uid());
			match raw.await {
				Ok(raw)=>{
					let _=self.push_cache(&raw).await;
					Ok(Some(raw.resolve(self)?))
				},
				Err(e)=>Err(match e{
					Some(e)=>format!("{}",e),
					None=>String::from("unknown error")
				})
			}
		}
	}
	pub async fn fetch_user(&self,uid:i32)->Result<RawUserData,Option<reqwest::Error>>{
		let contents=self.request(format!("https://enka.network/api/uid/{}/",uid)).await?;
		let lastupdate=SystemTime::now();
		Ok(RawUserData::from_raw(contents,uid,lastupdate))
	}
	async fn request(&self,url:impl AsRef<str>)->Result<Vec<u8>,Option<reqwest::Error>>{
		//println!("request {}",url.as_ref());
		let url=url.as_ref().to_owned();
		let mut request=match &self.client{
			Some(v)=>v.get(url),
			None=>return Err(None)
		};
		if let Some(header)=&self.header{
			request=request.headers(header.as_ref().to_owned());
		}
		let body=request.send().await?;
		let body=body.error_for_status()?;
		let body=body.bytes().await?;
		Ok(body.to_vec())
	}
	pub async fn push_cache(&self,data:&RawUserData)->std::io::Result<()>{
		self.user_cache.set(format!("{}",data.uid()),data.contents(),data.lastupdate()).await
		//let mut path=PathBuf::from(&self.user_cache_dir);
		//path.push(format!("{}",data.uid()));
		//crate::io::write_file(&path,data.contents(),data.lastupdate()).await
	}
	pub async fn find_cache(&self,uid:i32)->Option<RawUserData>{
		let (buf,modtime)=self.user_cache.get(format!("{}",uid)).await.ok()?;
		//let mut path=PathBuf::from(&self.user_cache_dir);
		//path.push(format!("{}",uid));
		//let (buf,modtime)=crate::io::read_file(&path).await.ok()?;
		Some(RawUserData::from_raw(buf.to_vec(),uid,modtime))
	}
	pub async fn assets(&self,name:impl AsRef<str>)->Result<Arc<Vec<u8>>,String>{
		let mut local_path=name.as_ref();
		let base_url=if local_path.starts_with("store/"){
			"https://github.com/EnkaNetwork/API-docs/raw/master/"
		}else if local_path.starts_with("ui/")||local_path.starts_with("img/")||local_path.starts_with("fonts/"){
			"https://enka.network/"
		}else if (local_path.starts_with("https://")||local_path.starts_with("http://"))&&!local_path.ends_with("/"){
			let scheme_last=local_path.find('/').unwrap()+2;
			let host=&local_path[scheme_last..];
			match host.find('/'){
				Some(index)=>{
					//let index=index+1;//path only
					//let base_url=&local_path[0..scheme_last+index];
					//local_path=&host[index..];
					let _=index;
					let base_url=&local_path[0..scheme_last];
					local_path=host;//host include
					base_url
				},
				None=>return Err(String::from("url error"))
			}
		}else{
			return Err(String::from("scheme error"));
		};
		let request_url=format!("{}{}",base_url,&local_path);
		let local_path=urlencoding::encode(local_path);
		let local_path=local_path.as_ref().replace("%2F","/");
		//println!("assets {}",&local_path);
		fn ioerror<T>(r:std::io::Result<T>,msg:impl AsRef<str>)->Result<T,String>{
			match r{
				Ok(v)=>Ok(v),
				Err(e)=>Err(format!("{}{}",msg.as_ref(),e))
			}
		}
		//let mut path=PathBuf::from(&self.assets_cache_dir);
		//path.push(Path::new(&local_path));
		//match crate::io::read_file(&path).await{
		match self.assets_cache.get(&local_path).await{
			Ok((buf,_))=>{
				Ok(buf)
			},
			Err(_)=>{
				let contents=match self.request(&request_url).await{
					Ok(v)=>v,
					Err(e)=>{
						return Err(format!("{:?}",e));
					}
				};
				ioerror(self.assets_cache.set(&local_path,&contents,SystemTime::now()).await,format!("create cache assets {:?}",&local_path))?;
				//ioerror(crate::io::write_file(&path,&contents,SystemTime::now()).await,format!("create cache assets {:?}",&path))?;
				Ok(Arc::new(contents))
			}
		}
	}
	pub async fn simple(&self,uid:i32)->Result<UserData,String>{
		match self.find_cache(uid).await{
			Some(cache)=>{
				let data=cache.resolve(self)?;
				if self.is_offline_mode(){
					Ok(data)
				}else{
					match self.reload(&data).await?{
						Some(new)=>Ok(new),
						None=>Ok(data)
					}
				}
			},
			None=>{
				let userdata=self.fetch_user(uid).await;
				match userdata {
					Ok(userdata)=>{
						let _=self.push_cache(&userdata).await;
						userdata.resolve(self)
					},
					Err(e)=>Err(match e{
						Some(e)=>format!("{}",e),
						None=>String::from("unknown error")
					})
				}
			}
		}
	}
	pub fn is_offline_mode(&self)->bool{
		self.client.is_none()
	}
	pub fn fight_prop_locale(&self,language:impl AsRef<str>)-> Result<FightPropLocale, String>{
		self.get_store()?.fight_prop_locale(language)
	}
}
pub(crate) async fn ui_image(path:impl AsRef<str>,api:&EnkaNetwork)->Result<DynamicImage,String>{
	let url=format!("ui/{}.png",path.as_ref());
	let bytes=api.assets(&url).await?;
	let bytes=bytes.as_ref();
	let reader=image::io::Reader::new(std::io::Cursor::new(bytes));
	let reader=match reader.with_guessed_format(){
		Ok(img)=>img,
		Err(e)=>return Err(format!("{}",e))
	};
	match reader.decode(){
		Ok(img)=>Ok(img),
		Err(e)=>Err(format!("{}",e))
	}
}
