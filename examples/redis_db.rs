
fn main(){
	let client=enkanetwork_rs::reqwest::Client::builder().user_agent("ExampleUserAgent").build().ok();
	let redis_client=redis::Client::open("redis://127.0.0.1/").unwrap();
	let disk_dir="./cache/".into();//disk cache
	let disk=match std::fs::create_dir_all(&disk_dir){
		Ok(_)=>Some(disk_dir),
		Err(_)=>None,
	};
	let cache=enkanetwork_rs::block_on(enkanetwork_rs::MemoryCache::from(disk,redis_client)).unwrap().unwrap();
	let mut api=enkanetwork_rs::EnkaNetwork::from(client,cache.clone(),cache);
	let api_copy=api.clone();
	api.set_store(enkanetwork_rs::block_on(async move{
		api_copy.store().await.ok()
	}).unwrap());
	enkanetwork_rs::block_on(async move{
		let user=api.simple(837338702).await;
		println!("{:?}",user);
	}).unwrap();
}
