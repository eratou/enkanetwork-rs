use std::collections::HashMap;

use serde_json::Value;

use crate::{EnkaNetwork, NameCard};
pub struct StoreValue{
	loc:Value,
	pub(crate)namecards:Value,
	pub(crate)characters:Value,
}
impl StoreValue{
	pub fn locale(&self,language:impl AsRef<str>,key:impl AsRef<str>)->Option<&str>{
		let map=self.loc.as_object()?;
		let lang=map.get(language.as_ref())?.as_object()?;
		lang.get(key.as_ref())?.as_str()
	}
	pub fn is_locale_available(&self,loc:impl AsRef<str>)->bool{
		match self.loc.as_object(){
			Some(v)=>{
				v.contains_key(loc.as_ref())
			},
			None=>false
		}
	}
	pub fn locale_list(&self)->Vec<&String>{
		match self.loc.as_object(){
			Some(v)=>{
				let keys=v.keys();
				let mut list=Vec::with_capacity(keys.len());
				for x in keys{
					list.push(x);
				}
				list
			},
			None=>vec![]
		}
	}
	pub fn namecard_path(&self,id:NameCard)->Result<&str,String>{
		let namecard_map=self.namecards.as_object().ok_or_else(||String::from("no namecards"))?;
		let json_value=namecard_map[&format!("{}",id.0)].as_object().ok_or_else(||String::from("not found in map"))?;
		json_value["icon"].as_str().ok_or_else(||String::from("not string"))
	}
}
async fn _store(api:&EnkaNetwork,path:impl AsRef<str>)->Result<Value,String>{
	let raw=api.assets(format!("store/{}",path.as_ref())).await?;
	let res:Result<Value,serde_json::Error> = serde_json::from_slice(&raw);
	match res {
		Ok(v)=>Ok(v),
		Err(e)=>Err(format!("{}",e))
	}
}
fn mearge_character_db(base:&mut Value,db:Value)->Option<()>{
	let mut db_map=HashMap::new();
	let dbroot=db.as_object()?;
	for (_,value) in dbroot{
		if let Some(side_icon)=value["namesideicon"].as_str(){
			db_map.insert(side_icon,value);
		}
	}
	let baseroot=base.as_object_mut()?;
	for (_,value) in baseroot{
		if let Some(side_icon)=value["SideIconName"].as_str(){
			if let Some(db_value)=db_map.get(side_icon){
				value["nameicon"]=db_value["nameicon"].clone();
				value["nameiconcard"]=db_value["nameiconcard"].clone();
				value["namegachasplash"]=db_value["namegachasplash"].clone();
				value["namegachaslice"]=db_value["namegachaslice"].clone();
			}
		}
	}
	Some(())
}
async fn characters(api:&EnkaNetwork)->Result<Value,String>{
	/*
	let mut path=PathBuf::from(&api.assets_cache_dir);
	path.push(Path::new("format_characters.json"));
	match File::open(&path){
		
		Ok(mut f)=>{
			let capacity=match f.metadata(){
				Ok(meta)=>meta.len() as usize,
				Err(_)=>0
			};
			let mut buf=Vec::with_capacity(capacity);
			return match f.read_to_end(&mut buf){
				Ok(_)=>{
					let cache:Result<Value,serde_json::Error> = serde_json::from_slice(&buf);
					match cache {
						Ok(v)=>Ok(v),
						Err(e)=>Err(format!("{}",e))
					}
				},
				Err(e)=>Err(format!("read cache assets {:?}{}",&path,e))
			}
		},
		Err(_)=>{}
	}
	*/
	let local_path="format_characters.json";
	match api.assets_cache.get(local_path).await{
		Ok((buf,_))=>{
			let cache:Result<Value,serde_json::Error> = serde_json::from_slice(&buf);
			return match cache {
				Ok(v)=>Ok(v),
				Err(e)=>Err(format!("{}",e))
			}
		},
		Err(_)=>{}
	}
	let mut base=_store(api,"characters.json").await?;
	let raw=api.assets("https://github.com/theBowja/genshin-db/raw/main/src/data/image/characters.json").await?;
	let db:Result<Value,serde_json::Error> = serde_json::from_slice(&raw);
	let db=match db {
		Ok(v)=>v,
		Err(e)=>return Err(format!("{}",e))
	};
	mearge_character_db(&mut base,db);
	let cache=if cfg!(debug_assertions) {
		serde_json::to_vec_pretty(&base)
	} else {
		serde_json::to_vec(&base)
	};
	if let Ok(json)=cache{
		if let Err(e)=api.assets_cache.set(local_path,&json,std::time::SystemTime::now()).await{
			println!("{}",e)
		}
		/*
		if let Err(e)=std::fs::write(&path,json){
			println!("{}",e)
		}
		*/
	}
	Ok(base)
}
pub(crate) async fn store(api:&EnkaNetwork)->Result<StoreValue,String>{
	let (loc_json,namecards_json,characters_json)=
	futures::join!(_store(api,"loc.json"),_store(api,"namecards.json"),characters(api));
	Ok(StoreValue{
		loc:loc_json?,
		namecards:namecards_json?,
		characters:characters_json?,
	})
}
