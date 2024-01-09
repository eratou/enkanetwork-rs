# enkanetwork-rs

Client library for the API provided at https://enka.network/  

## example  
Cargo.toml
```toml
[dependencies]
enkanetwork-rs={git="https://github.com/eratou/enkanetwork-rs.git",rev="02805cedc141da2f3ef60b769465b3d230dff913"}
```
main.rs
```rust
fn main(){
	let api=enkanetwork_rs::EnkaNetwork::new().unwrap();
	enkanetwork_rs::block_on(async move{
		match api.simple(837338702).await{
			Ok(data)=>{
				println!("{:?}",data);
			},
			Err(e)=>println!("{}",e)
		}
	}).unwrap();
}
```
[other examples](examples)

## features
| Name | Description |
| :------ | :--------------------------------------- |
|async-io | file access by [async-std](https://crates.io/crates/async-std) |
|text | text render utilities |
|vector-icon | svg icon utilities |
|redis-cache | memory cache replace to [redis](https://redis.io/) |

## target support
* [x] x86_64-pc-windows-msvc
* [x] x86_64-pc-windows-gnu
* [x] x86_64-unknown-linux-gnu
* [x] x86_64-unknown-linux-musl
* [x] i686-pc-windows-gnu
* [x] i686-unknown-linux-gnu
* [ ] i686-unknown-linux-musl
* [ ] wasm32-unknown-unknown
* [ ] aarch64-unknown-linux-musl
* [ ] aarch64-unknown-linux-gnu

## License
Apache 2.0 or MIT
