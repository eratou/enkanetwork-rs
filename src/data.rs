use std::collections::HashMap;
use std::time::{SystemTime, Duration};

use image::DynamicImage;
use serde_json::Value;

use crate::{EnkaNetwork, parse_character, Character, CharacterId};

#[derive(Clone)]
pub struct RawUserData{
	contents:Vec<u8>,
	uid:i32,
	lastupdate:SystemTime,
}
impl RawUserData{
	pub fn from_raw(contents:Vec<u8>,uid:i32,lastupdate:SystemTime)->Self{
		Self { contents,uid,lastupdate }
	}
	pub fn uid(&self)->i32{
		self.uid
	}
	pub fn lastupdate(&self)->SystemTime{
		self.lastupdate
	}
	pub(crate) fn contents(&self)->&[u8]{
		&self.contents
	}
	pub fn resolve(&self,api:&EnkaNetwork) -> Result<UserData,String> {
		let res:Result<Value,serde_json::Error> = serde_json::from_slice(&self.contents);
		match res{
			Ok(val)=>{
				match val.as_object() {
					Some(root)=>{
						let profile=PlayerInfo::from(root.get("playerInfo").ok_or_else(||String::from("no player info"))?);
						let mut characters=HashMap::new();
						if let Some(avatar_info_list)=root.get("avatarInfoList"){
							if let Some(map)=avatar_info_list.as_array(){
								for id in map{
									match parse_character(api,id){
										Ok(c)=>{
											characters.insert(c.id,c);
										}
										Err(e)=>{
											return Err(format!("parse character error {:?}\n{id}",e));
										}
									}
								}
							}
						}
						let ttl=match root.get("ttl"){
							Some(ttl)=>{
								ttl.as_u64().unwrap_or(5*60) as u32
							},
							None=>5*60
						};
						Ok(UserData{
							uid:self.uid,
							ttl,
							lastupdate:self.lastupdate,
							profile,
							characters
						})
					},
					None=>Err(String::from("NoRootValue"))
				}
			},
			Err(e)=>Err(format!("{}",e))
		}
	}
}
#[derive(Clone,Debug)]
pub struct UserData{
	uid:i32,
	ttl:u32,
	lastupdate:SystemTime,
	profile:PlayerInfo,
	characters:HashMap<CharacterId,Character>,
}
impl UserData{
	pub fn reload_time(&self)->SystemTime{
		self.lastupdate+Duration::new(self.ttl as u64,0)
	}
	pub fn lastupdate(&self)->SystemTime{
		self.lastupdate
	}
	pub fn profile(&self)->&PlayerInfo{
		&self.profile
	}
	pub fn character(&self,id:CharacterId)->Option<&Character>{
		self.characters.get(&id)
	}
	pub fn uid(&self)->i32{
		self.uid
	}
	pub fn ttl_sec(&self)->u32{
		self.ttl
	}
}
#[derive(Clone,Eq,PartialEq,Debug)]
pub struct PlayerInfo{
	nickname:String,
	signature:String,
	level:u8,
	world_level:u8,
	achievement:u32,
	tower_floor_index:u8,
	tower_level_index:u8,
	name_card:NameCard,
	profile_picture:CharacterId,
	avatar_info_list:Vec<CharacterId>,
	name_card_list:Vec<NameCard>,
}
impl PlayerInfo{
	fn from(v:&Value)->Self{
		use crate::get_or_null as get;
		//println!("parse_profile{:?}",v);
		let mut avatar_info_list=vec![];
		if let Some(arr)=get(v,"showAvatarInfoList").as_array(){
			for item in arr{
				if let Some(id)=get(item,"avatarId").as_u64(){
					avatar_info_list.push(CharacterId(id as u32));
				}else{
					//println!("{:?}",item);
				};
			}
		}
		let mut name_card_list=vec![];
		if let Some(arr)=get(v,"showNameCardIdList").as_array(){
			for item in arr{
				if let Some(id)=item.as_u64(){
					name_card_list.push(NameCard(id as u32));
				}
			}
		}
		Self{
			nickname:get(v,"nickname").as_str().unwrap_or("").to_owned(),
			signature:get(v,"signature").as_str().unwrap_or("").to_owned(),
			level:get(v,"level").as_u64().unwrap_or(0) as u8,
			world_level:get(v,"worldLevel").as_u64().unwrap_or(0) as u8,
			achievement:get(v,"finishAchievementNum").as_u64().unwrap_or(0) as u32,
			name_card:NameCard(get(v,"nameCardId").as_u64().unwrap_or(0) as u32),
			tower_floor_index:get(v,"towerFloorIndex").as_u64().unwrap_or(0) as u8,
			tower_level_index:get(v,"towerLevelIndex").as_u64().unwrap_or(0) as u8,
			profile_picture:CharacterId(get(get(v,"profilePicture").as_ref(),"avatarId").as_i64().unwrap_or(0) as u32),
			avatar_info_list,
			name_card_list
		}
		//CharacterId(v["avatarId"].as_u64().unwrap_or(0) as u32)
	}
	pub fn nickname(&self)->&String{
		&self.nickname
	}
	pub fn signature(&self)->&String{
		&self.signature
	}
	pub fn level(&self)->u8{
		self.level
	}
	pub fn world_level(&self)->u8{
		self.world_level
	}
	pub fn achievement(&self)->u32{
		self.achievement
	}
	pub fn name_card(&self)->NameCard{
		self.name_card
	}
	pub fn profile_picture(&self)->CharacterId{
		self.profile_picture
	}
	pub fn show_character_list(&self)->&Vec<CharacterId>{
		&self.avatar_info_list
	}
	pub fn show_name_card_list(&self)->&Vec<NameCard>{
		&self.name_card_list
	}
}
#[derive(Copy,Clone,Eq,PartialEq,Debug)]
pub struct NameCard(pub(crate)u32);
impl NameCard{
	pub fn has_value(&self)->bool{
		self.0>0
	}
	pub async fn image(&self,api:&EnkaNetwork)->Result<DynamicImage,String>{
		if !self.has_value(){
			return Err(String::from("None"));
		}
		let store=api.get_store()?;
		crate::api::ui_image(store.namecard_path(*self)?,api).await
	}
}
