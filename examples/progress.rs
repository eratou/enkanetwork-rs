fn main(){
	let api=enkanetwork_rs::EnkaNetwork::new().unwrap();
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
