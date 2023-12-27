use std::{collections::HashMap, str::FromStr, borrow::Cow, fmt::Display};

use image::DynamicImage;
use serde_json::Value;

use crate::{EnkaNetwork, ui_image, FightProp, StoreValue};

#[derive(Clone,Debug)]
pub struct Character{
	pub id:CharacterId,
	pub element:Element,
	name:u32,
	talents:Vec<CharacterTalent>,
	skills:Vec<CharacterSkill>,
	reliquarys:Vec<Reliquary>,
	weapon:Weapon,
	costumes:HashMap<u32,Costume>,
	current_costume:Option<u64>,
	gachaslice:Option<String>,
	gachasplash:Option<String>,
	icon:Option<String>,
	iconcard:Option<String>,
	fight_prop:FightProp,
	friendship:u8,
	pub level:u8,
	pub ascension:u8,
	pub xp:u32,
}
impl Character{
	pub fn consts(&self)->&Vec<CharacterTalent>{
		&self.talents
	}
	pub fn name<'a>(&self,api:&'a EnkaNetwork,language:impl AsRef<str>)->Result<&'a str,String>{
		let s=api.get_store()?;
		s.locale(language,&format!("{}",self.name)).ok_or_else(||String::from("no locale data"))
	}
	pub fn costumes(&self)->Vec<&Costume>{
		self.costumes.values().collect()
	}
	pub fn current_costume(&self)->Option<&Costume>{
		self.costumes.get(&(self.current_costume? as u32))
	}
	pub fn name_gacha_slice(&self)->Option<&String>{
		self.gachaslice.as_ref()
	}
	pub fn name_gacha_splash(&self)->Option<&String>{
		self.gachasplash.as_ref()
	}
	pub fn name_icon(&self)->Option<&String>{
		self.icon.as_ref()
	}
	pub fn name_iconcard(&self)->Option<&String>{
		self.iconcard.as_ref()
	}
	pub fn skills(&self)->&Vec<CharacterSkill>{
		&self.skills
	}
	pub async fn image_icon(&self,api:&EnkaNetwork)->Option<DynamicImage>{
		ui_image(self.name_icon()?,api).await.ok()
	}
	pub async fn image_iconcard(&self,api:&EnkaNetwork)->Option<DynamicImage>{
		ui_image(self.name_iconcard()?,api).await.ok()
	}
	pub async fn image_gacha_slice(&self,api:&EnkaNetwork)->Option<DynamicImage>{
		let name=match self.name_gacha_slice(){
			Some(p)=>Cow::Borrowed(p),
			None=>Cow::Owned(self.icon.as_ref()?.replace("AvatarIcon","Gacha_AvatarImg"))
		};
		ui_image(name.as_ref(),api).await.ok()
	}
	pub async fn image_gacha_splash(&self,api:&EnkaNetwork)->Option<DynamicImage>{
		let name=match self.name_gacha_splash(){
			Some(p)=>Cow::Borrowed(p),
			None=>Cow::Owned(self.icon.as_ref()?.replace("AvatarIcon","Gacha_AvatarImg"))
		};
		ui_image(name.as_ref(),api).await.ok()
	}
	pub fn friendship(&self)->u8{
		match self.id.0{
			10000005=>0,//player=0
			10000007=>0,//player=0
			_=>self.friendship
		}
	}
	pub fn fight_prop(&self)->&FightProp{
		&self.fight_prop
	}
	pub fn reliquarys(&self)->&Vec<Reliquary>{
		&self.reliquarys
	}
	pub fn weapon(&self)->&Weapon{
		&self.weapon
	}
	pub fn ascension_level(&self)->u8{
		ascension_level_map(self.ascension)
	}
}
#[derive(Debug)]
pub(crate) enum CharacterParseError{
	NoAvatarID,
	InvalidAvatarID,
	NoStore(String),
	NoStoreAvatar,
	InvalidStoreAvatar,
	NoSkills,
	InvalidSkillId,
	NoPropMap,
	Unknown,
}
#[derive(Hash,Copy,Clone,Eq,PartialEq,Debug)]
pub struct CharacterId(pub u32);
pub(crate) fn parse_character(api:&EnkaNetwork,player_character:&Value)->Result<Character,CharacterParseError>{
	let avatar_id=player_character.get("avatarId");
	let avatar_id=avatar_id.ok_or(CharacterParseError::NoAvatarID)?;
	let avatar_id=avatar_id.as_u64().ok_or(CharacterParseError::InvalidAvatarID)? as u32;
	//println!("player data {:?}",id);
	let character_id_str=if avatar_id==10000005||avatar_id==10000007{
		let depot_id=match player_character.get("skillDepotId"){
			Some(depot_id)=>depot_id.to_string(),
			None=>format!("{}01",avatar_id.to_string().get(7..7).unwrap())
		};
		format!("{}-{}",avatar_id,depot_id)
	}else{
		format!("{}",avatar_id)
	};
	let store=api.get_store().map_err(|s|CharacterParseError::NoStore(s))?;
	let characters=store.characters.get(character_id_str);
	let characters=characters.ok_or(CharacterParseError::NoStoreAvatar)?.as_object();
	let characters=characters.ok_or(CharacterParseError::InvalidStoreAvatar)?;
	let mut talents=vec![];
	if let Some(consts)=characters.get("Consts"){
		if let Some(consts)=consts.as_array() {
			let talent_count={
				match player_character.get("talentIdList"){
					Some(consts)=>{
						match consts.as_array(){
							Some(arr)=>arr.len(),
							None=>0
						}
					},
					None=>0
				}
			};
			let mut index=0;
			for name in consts{
				let talent=CharacterTalent{
					image:name.as_str().map(|s|s.to_owned()).unwrap_or_default(),
					unlock:index<talent_count
				};
				index+=1;
				talents.push(talent);
			}
		}
	}
	fn find(map:&serde_json::Map<String, Value>,key:&str)->Option<String>{
		Some(String::from(map.get(key)?.as_str()?))
	}
	let mut skills=vec![];
	{
		let extra_map=crate::get_or_null(player_character,"proudSkillExtraLevelMap");
		let skill_level_map=crate::get_or_null(player_character,"skillLevelMap");
		let skill_level_map=skill_level_map.as_object();
		if let Some(skill_level_map)=skill_level_map{
			let skill_image=characters.get("Skills").ok_or(CharacterParseError::NoSkills)?;
			let mut skill_map=HashMap::with_capacity(4);
			for (id,level) in skill_level_map{
				let image=match skill_image.get(id){
					Some(v)=>{
						match v.as_str() {
							Some(str)=>Some(str.to_owned()),
							None=>None
						}
					},None=>None
				};
				let extra_level=match characters.get("ProudMap"){
					Some(map)=>{
						match map.get(id){
							Some(id)=>{
								match extra_map.get(id.to_string()){
									Some(v)=>{
										match v.as_u64(){
											Some(v)=>v as u8,
											None=>0
										}
									},None=>0
								}
							},None=>0
						}
					},None=>0
				};
				let id=u64::from_str(id).map_err(|_|CharacterParseError::InvalidSkillId)? as u32;
				let s=CharacterSkill{
					extra_level,
					id,
					level:level.as_u64().unwrap_or_default() as u8,
					image
				};
				skill_map.insert(id,s);
			}
			let skill_order=characters.get("SkillOrder");
			if let Some(skill_order)=skill_order{
				if let Some(skill_order)=skill_order.as_array(){
					for skill_id in skill_order{
						if let Some(skill_id)=skill_id.as_u64(){
							if let Some(s)=skill_map.remove(&(skill_id as u32)){
								skills.push(s);
							}
						}
					}
				}
			}
		}
	}
	let current_costume=match player_character.get("costumeId"){
		Some(json)=>json.as_u64(),
		None=>None
	};
	let mut costumes=HashMap::new();
	if let Some(costumes_list)=characters.get("Costumes"){
		if let Some(costumes_list)=costumes_list.as_object(){
			for (id,costume_json) in costumes_list{
				fn load_costume(id:&String,useing:bool,json:&Value)->Option<Costume>{
					fn str_or(json:&Value,key:&str)->Option<String>{
						Some(json.get(key)?.as_str()?.to_owned())
					}
					Some(Costume{
						id:u32::from_str(id).ok()?,
						useing,
						art:str_or(json,"art"),
						icon:str_or(json,"icon"),
						side_icon:str_or(json,"sideIconName")
					})
				}
				let is_useing=match current_costume{
					Some(costume_id)=>costume_id.to_string().as_str()==id,
					None=>false
				};
				if let Some(c)=load_costume(id,is_useing,costume_json){
					costumes.insert(c.id,c);
				}
			}
		}
	}
	let friendship=match player_character.get("fetterInfo"){
		Some(fetter_info)=>{
			match fetter_info.get("expLevel"){
				Some(exp)=>{
					match exp.as_u64(){
						Some(friendship)=>friendship as u8,
						None=>1u8
					}
				},
				None=>1u8
			}
		},
		None=>1u8
	};
	let prop_map=player_character.get("propMap").ok_or(CharacterParseError::NoPropMap)?;
	fn prop(prop_map:&Value,key:&str)->Option<u64>{
		let s=prop_map.get(key)?.get("val")?.as_str()?;
		u64::from_str(s).ok()
	}
	let equip_list=player_character.get("equipList").ok_or(CharacterParseError::Unknown)?;
	let equip_list=equip_list.as_array().ok_or(CharacterParseError::Unknown)?;
	let (weapon,reliquarys)=parse_equip_list(equip_list);
	let fight_prop=FightProp::from_json(player_character.get("fightPropMap").ok_or(CharacterParseError::Unknown)?);
	let element=characters.get("Element").ok_or(CharacterParseError::Unknown)?.as_str().ok_or(CharacterParseError::Unknown)?;
	let element=Element::from_str(element).ok().ok_or(CharacterParseError::Unknown)?;
	let name=characters.get("NameTextMapHash").ok_or(CharacterParseError::Unknown)?.as_u64().ok_or(CharacterParseError::Unknown)? as u32;
	let weapon=weapon.ok_or(CharacterParseError::Unknown)?;
	Ok(Character{
		id:CharacterId(avatar_id),
		fight_prop,
		element,
		name,
		talents,
		skills,
		reliquarys,
		weapon,
		costumes,
		current_costume,
		gachaslice:find(characters,"namegachaslice"),
		gachasplash:find(characters,"namegachasplash"),
		icon:find(characters,"nameicon"),
		iconcard:find(characters,"nameiconcard"),
		friendship,
		xp:prop(prop_map,"1001").unwrap_or(0) as u32,
		ascension:prop(prop_map,"1002").unwrap_or(0) as u8,
		level:prop(prop_map,"4001").unwrap_or(0) as u8,
	})
}
#[derive(Hash,Copy,Clone,Eq,PartialEq,Debug)]
pub enum Element{
	Fire,
	Water,
	Wind,
	Electric,
	Grass,
	Ice,
	Rock,
	None
}
impl FromStr for Element {
	type Err = String;
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"Fire" => Ok(Self::Fire),
			"Water" => Ok(Self::Water),
			"Wind" => Ok(Self::Wind),
			"Electric" => Ok(Self::Electric),
			"Grass" => Ok(Self::Grass),
			"Ice" => Ok(Self::Ice),
			"Rock" => Ok(Self::Rock),
			"Physical" => Ok(Self::None),
			"None" => Ok(Self::None),
			_ => Err(String::from("unknown element"))
		}
	}
}
impl ToString for Element{
	fn to_string(&self)->String{
		format!("{:?}",self)
	}
}
impl Element{
	pub fn color_rgb(&self)->[u8;3]{
		match self{
			Self::Fire=>[255,153,85],
			Self::Water=>[62,153,255],
			Self::Wind=>[128,255,230],
			Self::Electric=>[179,128,255],
			Self::Grass=>[165,200,59],
			Self::Ice=>[85,221,255],
			Self::Rock=>[255,204,0],
			Self::None=>[255,255,255],
		}
	}
	pub fn fight_prop_name(&self)->&'static str{
		match self {
			Self::Fire=>"FIRE",
			Self::Water=>"WATER",
			Self::Wind=>"WIND",
			Self::Electric=>"ELEC",
			Self::Grass=>"GRASS",
			Self::Ice=>"ICE",
			Self::Rock=>"ROCK",
			Self::None=>"PHYSICAL",
		}
	}
	pub fn attack_name<'a>(&self,store:&'a StoreValue,language: impl AsRef<str>)->&'a str{
		let res=store.locale(language,format!("FIGHT_PROP_{}_ADD_HURT",self.fight_prop_name()));
		match res{
			Some(v)=>v,
			None=>""
		}
	}
	pub fn resist_name<'a>(&self,store:&'a StoreValue,language: impl AsRef<str>)->&'a str{
		let res=store.locale(language,format!("FIGHT_PROP_{}_SUB_HURT",self.fight_prop_name()));
		match res{
			Some(v)=>v,
			None=>""
		}
	}
}
//命の星座
#[derive(Clone,Eq,PartialEq,Debug)]
pub struct CharacterTalent{
	image:String,
	unlock:bool
}
impl CharacterTalent{
	pub fn is_unlock(&self)->bool{
		self.unlock
	}
	pub fn path(&self)->&String{
		&self.image
	}
	pub async fn image(&self,api:&EnkaNetwork)->Result<DynamicImage,String>{
		if self.image.is_empty(){
			return Err(String::from("None"));
		}
		ui_image(&self.image,api).await
	}
}
#[derive(Clone,Eq,PartialEq,Debug)]
pub struct Costume{
	id:u32,
	useing:bool,
	art:Option<String>,
	icon:Option<String>,
	side_icon:Option<String>,
}
impl Costume{
	pub async fn image_art(&self,api:&EnkaNetwork)->Option<DynamicImage>{
		ui_image(self.art.as_ref()?,api).await.ok()
	}
	pub async fn image_icon(&self,api:&EnkaNetwork)->Option<DynamicImage>{
		ui_image(self.icon.as_ref()?,api).await.ok()
	}
	pub async fn image_side_icon(&self,api:&EnkaNetwork)->Option<DynamicImage>{
		ui_image(self.side_icon.as_ref()?,api).await.ok()
	}
	pub fn is_useing(&self)->bool{
		self.useing
	}
}
#[derive(Clone,Eq,PartialEq,Debug)]
pub struct CharacterSkill{
	id:u32,
	level:u8,
	extra_level:u8,
	image:Option<String>,
}
impl CharacterSkill{
	pub fn level(&self)->u8{
		self.level
	}
	pub fn extra_level(&self)->u8{
		self.extra_level
	}
	pub async fn image(&self,api:&EnkaNetwork)->Result<DynamicImage,String>{
		match &self.image{
			Some(path)=>ui_image(path,api).await,
			None=>Err("no image".to_string())
		}
	}
}
#[derive(Hash,Copy,Clone,Debug)]
pub enum Stats{
	Hp,
	Attack,
	Defense,
	HpPercent,
	AttackPercent,
	DefensePercent,
	Critical,
	CriticalHurt,
	ChargeEfficiency,
	Heal,
	ElementMastery,
	ElementAddHurt(Element),
	None
}
impl Stats{
	fn parse(t:impl AsRef<str>)->Self{
		match t.as_ref() {
			"FIGHT_PROP_HP"=>Self::Hp,
			"FIGHT_PROP_ATTACK"=>Self::Attack,
			"FIGHT_PROP_DEFENSE"=>Self::Defense,
			"FIGHT_PROP_HP_PERCENT"=>Self::HpPercent,
			"FIGHT_PROP_ATTACK_PERCENT"=>Self::AttackPercent,
			"FIGHT_PROP_DEFENSE_PERCENT"=>Self::DefensePercent,
			"FIGHT_PROP_CRITICAL"=>Self::Critical,
			"FIGHT_PROP_CRITICAL_HURT"=>Self::CriticalHurt,
			"FIGHT_PROP_CHARGE_EFFICIENCY"=>Self::ChargeEfficiency,
			"FIGHT_PROP_HEAL_ADD"=>Self::Heal,
			"FIGHT_PROP_ELEMENT_MASTERY"=>Self::ElementMastery,
			"FIGHT_PROP_PHYSICAL_ADD_HURT"=>Self::ElementAddHurt(Element::None),
			"FIGHT_PROP_FIRE_ADD_HURT"=>Self::ElementAddHurt(Element::Fire),
			"FIGHT_PROP_ELEC_ADD_HURT"=>Self::ElementAddHurt(Element::Electric),
			"FIGHT_PROP_WATER_ADD_HURT"=>Self::ElementAddHurt(Element::Water),
			"FIGHT_PROP_WIND_ADD_HURT"=>Self::ElementAddHurt(Element::Wind),
			"FIGHT_PROP_ICE_ADD_HURT"=>Self::ElementAddHurt(Element::Ice),
			"FIGHT_PROP_ROCK_ADD_HURT"=>Self::ElementAddHurt(Element::Rock),
			"FIGHT_PROP_GRASS_ADD_HURT"=>Self::ElementAddHurt(Element::Grass),
			_=>Self::None
		}
	}
	pub fn id(&self)->Cow<str>{
		match self{
			Self::Hp=>Cow::Borrowed("FIGHT_PROP_HP"),
			Self::Attack=>Cow::Borrowed("FIGHT_PROP_ATTACK"),
			Self::Defense=>Cow::Borrowed("FIGHT_PROP_DEFENSE"),
			Self::HpPercent=>Cow::Borrowed("FIGHT_PROP_HP_PERCENT"),
			Self::AttackPercent=>Cow::Borrowed("FIGHT_PROP_ATTACK_PERCENT"),
			Self::DefensePercent=>Cow::Borrowed("FIGHT_PROP_DEFENSE_PERCENT"),
			Self::Critical=>Cow::Borrowed("FIGHT_PROP_CRITICAL"),
			Self::CriticalHurt=>Cow::Borrowed("FIGHT_PROP_CRITICAL_HURT"),
			Self::ChargeEfficiency=>Cow::Borrowed("FIGHT_PROP_CHARGE_EFFICIENCY"),
			Self::Heal=>Cow::Borrowed("FIGHT_PROP_HEAL_ADD"),
			Self::ElementMastery=>Cow::Borrowed("FIGHT_PROP_ELEMENT_MASTERY"),
			Self::ElementAddHurt(e)=>Cow::Owned(format!("FIGHT_PROP_{}_ADD_HURT",e.fight_prop_name())),
			_=>Cow::Borrowed("")
		}
	}
	pub fn name<'a>(&self,api:&'a crate::EnkaNetwork,language:impl AsRef<str>)->Option<&'a str>{
		api.get_store().ok()?.locale(language,self.id())
	}
}
#[derive(Copy,Clone,Debug)]
pub struct StatsValue(pub Stats,pub f64);
impl Display for StatsValue{
	fn fmt(&self,f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self.0{
			Stats::Hp|
			Stats::Attack|
			Stats::Defense|
			Stats::ElementMastery|
			Stats::None =>write!(f,"{}",self.1.round() as u64),
			_=>write!(f,"{:.*}%",1,self.1)
		}
	}
}
#[derive(Hash,Copy,Clone,Debug)]
pub enum ReliquaryType{
	Flower,
	Feather,
	Sands,
	Goblet,
	Circlet,
}
impl ReliquaryType{
	fn parse(t:impl AsRef<str>)->Result<Self,()>{
		Ok(match t.as_ref() {
			"EQUIP_BRACER"=>Self::Flower,
			"EQUIP_NECKLACE"=>Self::Feather,
			"EQUIP_SHOES"=>Self::Sands,
			"EQUIP_RING"=>Self::Goblet,
			"EQUIP_DRESS"=>Self::Circlet,
			_=>return Err(())
		})
	}
}
#[derive(Clone,Debug)]
pub struct Reliquary{
	name:u32,
	set_name:u32,
	icon:String,
	pub position:ReliquaryType,
	pub id:u32,
	pub level:u8,
	pub rarity:u8,
	pub main_stats:StatsValue,
	pub sub_stats:[Option<StatsValue>;4],
}
impl Reliquary{
	pub fn set_name_hash(&self)->u32{
		self.set_name
	}
	pub fn set_name<'a>(&self,api:&'a crate::EnkaNetwork,language:impl AsRef<str>)->Option<&'a str>{
		api.get_store().ok()?.locale(language,self.set_name.to_string())
	}
	pub fn name_hash(&self)->u32{
		self.name
	}
	pub fn name<'a>(&self,api:&'a crate::EnkaNetwork,language:impl AsRef<str>)->Option<&'a str>{
		api.get_store().ok()?.locale(language,self.name.to_string())
	}
	pub fn name_icon(&self)->&String{
		&self.icon
	}
	pub async fn image_icon(&self,api:&EnkaNetwork)->Result<DynamicImage, String>{
		crate::ui_image(self.name_icon(),api).await
	}
}
#[derive(Clone,Debug)]
pub struct Weapon{
	name:u32,
	icon:String,
	pub id:u32,
	pub level:u8,
	pub ascension:u8,
	pub refinement:u8,
	pub rarity:u8,
	pub base_attack:i32,
	pub stats:Option<StatsValue>,
}
impl Weapon{
	pub fn name_hash(&self)->u32{
		self.name
	}
	pub fn name<'a>(&self,api:&'a crate::EnkaNetwork,language:impl AsRef<str>)->Option<&'a str>{
		api.get_store().ok()?.locale(language,self.name.to_string())
	}
	pub fn name_icon(&self)->&String{
		&self.icon
	}
	pub async fn image_icon(&self,api:&EnkaNetwork)->Result<DynamicImage, String>{
		let name=if self.ascension>1{
			Cow::Owned(format!("{}_Awaken",self.name_icon()))
		}else{
			Cow::Borrowed(self.name_icon())
		};
		crate::ui_image(name.as_ref(),api).await
	}
	pub fn ascension_level(&self)->u8{
		ascension_level_map(self.ascension)
	}
}
fn ascension_level_map(lv:u8)->u8{
	match lv{
		0=>20,
		1=>40,
		2=>50,
		3=>60,
		4=>70,
		5=>80,
		6=>90,
		_=>0
	}
}
fn parse_equip_list(list:&Vec<Value>)->(Option<Weapon>,Vec<Reliquary>){
	let mut weapon=None;
	let mut reliquarys=vec![];
	for entry in list{
		if entry.get("reliquary").is_some(){
			if let Some(r)=parse_equip_reliquary(entry){
				reliquarys.push(r);
			}
		}else if entry.get("weapon").is_some(){
			if let Some(w)=parse_equip_weapon(entry){
				weapon=Some(w);
			}
		}
	}
	(weapon,reliquarys)
}
fn parse_equip_reliquary(entry:&Value)->Option<Reliquary>{
	let id=entry.get("itemId")?.as_u64()? as u32;
	let reliquary=entry.get("reliquary")?;
	let flat=entry.get("flat")?;
	let reliquary_substats=flat.get("reliquarySubstats");
	let mut sub_stats=[None,None,None,None];
	if let Some(reliquary_substats)=reliquary_substats{
		if let Some(reliquary_substats)=reliquary_substats.as_array(){
			let mut index=0;
			for v in reliquary_substats{
				sub_stats[index]=parse_reliquary_stat(v,"appendPropId");
				index+=1;
			}
		}
	}
	Some(Reliquary{
		id,
		position:ReliquaryType::parse(flat.get("equipType")?.as_str()?).ok()?,
		icon:flat.get("icon")?.as_str()?.to_owned(),
		name:u32::from_str(flat.get("nameTextMapHash")?.as_str()?).ok()?,
		set_name:u32::from_str(flat.get("setNameTextMapHash")?.as_str()?).ok()?,
		rarity:flat.get("rankLevel")?.as_u64()? as u8,
		level:(reliquary.get("level")?.as_u64()?-1) as u8,
		main_stats:parse_reliquary_stat(flat.get("reliquaryMainstat")?,"mainPropId")?,
		sub_stats,
	})
}
fn parse_reliquary_stat(v:&Value,id_key:&str)->Option<StatsValue>{
	Some(StatsValue(Stats::parse(v.get(id_key)?.as_str()?),v.get("statValue")?.as_f64()?))
}
fn parse_equip_weapon(entry:&Value)->Option<Weapon>{
	let id=entry.get("itemId")?.as_u64()? as u32;
	let weapon=entry.get("weapon")?;
	let flat=entry.get("flat")?;
	let stats_list=flat.get("weaponStats")?.as_array()?;
	let mut base_attack=0;
	let mut stats=None;
	for v in stats_list{
		let prop_name=v.get("appendPropId")?.as_str()?;
		let value=v.get("statValue")?.as_f64()?;
		match prop_name{
			"FIGHT_PROP_BASE_ATTACK"=>base_attack=value as i32,
			a=>stats=Some(StatsValue(Stats::parse(a),value)),
		}
	}
	Some(Weapon{
		id,
		level:weapon.get("level")?.as_u64()? as u8,
		ascension:weapon.get("promoteLevel")?.as_u64()? as u8,
		refinement:weapon.get("affixMap")?.get(format!("1{}",id))?.as_u64()? as u8,
		rarity:flat.get("rankLevel")?.as_u64()? as u8,
		icon:flat.get("icon")?.as_str()?.to_owned(),
		name:u32::from_str(flat.get("nameTextMapHash")?.as_str()?).ok()?,
		base_attack,
		stats
	})
}
