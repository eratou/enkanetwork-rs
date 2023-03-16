use enkanetwork_rs::MemoryCache;

#[cfg(feature="redis")]
fn caches()->Result<(MemoryCache,MemoryCache),impl std::fmt::Debug>{
	let client = enkanetwork_rs::redis::Client::open("redis://127.0.0.1/")?;
	let rt=tokio::runtime::Builder::new_current_thread().enable_all().build()?;
	let cache=rt.block_on(async move{
		MemoryCache::from(Some(String::from("./cache/")), client).await
	})?;
	Ok::<(MemoryCache, MemoryCache),enkanetwork_rs::redis::RedisError>((cache.clone(),cache))
}
#[cfg(not(feature="redis"))]
fn caches()->Result<(MemoryCache,MemoryCache),impl std::fmt::Debug>{
	Ok::<(MemoryCache, MemoryCache),std::io::Error>((MemoryCache::new(String::from("./cache/assets/"))?,
	MemoryCache::new(String::from("./cache/u/"))?))
}
fn main(){
	let client=enkanetwork_rs::reqwest::Client::builder().user_agent("ExampleUserAgent").build().ok();
	let (assets_cache,user_cache)=caches().unwrap();
	let mut api=enkanetwork_rs::EnkaNetwork::from(client,assets_cache,user_cache);
	let api_copy=api.clone();
	api.set_store(enkanetwork_rs::block_on(async move{
		api_copy.store().await.ok()
	}).unwrap());
	let api_ref=api.clone();
	let job_a=async move{
		let _=api_ref.simple(700378769).await;
		println!("a");
	};
	let mut jobs=vec![];
	for i in 0..15{
		let api_ref=api.clone();
		jobs.push(async move{
			let _=api_ref.simple(837338702).await;
			println!("{}",i);
		});
	}
	enkanetwork_rs::block_on(async{
		futures::join!(job_a,futures::future::join_all(jobs));
	}).unwrap();
}
