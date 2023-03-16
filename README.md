# enkanetwork-rs

https://enka.network/ で提供されるAPIのクライアントライブラリ  

## 例  
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
[その他の実装例](examples)

## features
| 名前 | 説明 |
| :------ | :--------------------------------------- |
|async-io | ファイル操作に[async-std](https://crates.io/crates/async-std)を使用します |
|text | テキストの描画を補助する機能を有効にします |
|vector-icon | svgアイコンの描画を補助する機能を有効にします |
|redis-cache | メモリキャッシュをredisに置き換えます |

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
