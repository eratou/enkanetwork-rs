mod api;
mod data;
mod store;
mod fight_prop;
mod character;
mod io;
use futures::Future;
pub use io::MemoryCache;
pub use data::*;
pub use api::*;
pub use fight_prop::*;
pub use character::*;
pub use icon::IconData;
pub use store::*;
pub use reqwest;

#[cfg(feature="redis-cache")]
pub use redis;

#[cfg(feature="text")]
mod textrender;

#[cfg(feature="vector-icon")]
mod icon;

pub const USER_AGENT:&'static str="enkanetwork.rs/v0.0.2";

#[cfg(test)]
mod tests {
	use std::io::Cursor;
	use super::*;
	#[test]
	fn it_works() {
		futures::executor::block_on(async{
			let client=reqwest::Client::builder().user_agent(crate::USER_AGENT).build().ok();
			let assets_cache=MemoryCache::new("./cache/assets/".into()).unwrap();
			let user_cache=MemoryCache::new("./cache/u/".into()).unwrap();
			let mut api=EnkaNetwork::from(client,assets_cache,user_cache);
			let api_copy=api.clone();
			api.set_store(block_on(async move{
				api_copy.store().await.ok()
			}).unwrap());
			let bytes=api.assets("https://cdn.discordapp.com/attachments/555934591056085013/939543539073814598/about.png").await.unwrap();
			let bytes=bytes.as_ref();
			let read=image::io::Reader::new(Cursor::new(bytes));
			read.with_guessed_format().unwrap().decode().unwrap().save("x.png").unwrap();
			let cp=api.clone();
			std::thread::spawn(move ||{
				futures::executor::block_on(async{
					println!("{:?}",cp.simple(12345).await);
				});
			});
			println!("{:?}",api.simple(12345).await);
		});
	}
}
#[derive(Debug)]
pub enum PoolExecuteError{
	TokioRuntime(std::io::Error),
	CreatePool(std::io::Error),
	Unknown,
}
#[cfg(not(target_arch = "wasm32"))]
pub fn block_on<Fut>(future: Fut)->Result<Fut::Output,PoolExecuteError> where Fut:Future+Send+'static,Fut::Output:Send{
	let rt=tokio::runtime::Builder::new_multi_thread().enable_all().build();
	let rt=match rt{
		Ok(v)=>v,
		Err(e)=>return Err(PoolExecuteError::TokioRuntime(e))
	};
	Ok(rt.block_on(future))
}
#[cfg(target_arch = "wasm32")]
pub fn block_on<Fut>(future: Fut)->Result<Fut::Output,PoolExecuteError> where Fut:Future+'static,Fut::Output:Send{
	let rt=tokio::runtime::Builder::new_current_thread().enable_all().build();
	let rt=match rt{
		Ok(v)=>v,
		Err(e)=>return Err(PoolExecuteError::TokioRuntime(e))
	};
	Ok(rt.block_on(future))
}
/*
//this code network io error
pub fn _block_on<Fut>(num_threads:usize,future: Fut)->Result<Fut::Output,PoolExecuteError> where Fut:Future+Send+'static,Fut::Output:Send{
	let mut build=ThreadPool::builder();
	if num_threads>0{
		build.pool_size(num_threads);
	}
	let pool=match build.create(){
		Ok(pool)=>pool,
		Err(e)=>return Err(PoolExecuteError::CreatePool(e))
	};
	Ok(futures::executor::block_on({
		match pool.spawn_with_handle(future){
			Ok(handle)=>handle,
			Err(e)=>return Err(PoolExecuteError::Spawn(e))
		}
	}))
}
*/
pub(crate) fn get_or_null<'a>(v:&'a serde_json::Value,key:&str)->std::borrow::Cow<'a,serde_json::Value>{
	match v.get(key){
		Some(v)=>std::borrow::Cow::Borrowed(v),
		None=>std::borrow::Cow::Owned(serde_json::Value::Null)
	}
}
