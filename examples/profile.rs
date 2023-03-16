use std::{sync::Arc, collections::HashMap};

use chrono::{DateTime,Local};
use enkanetwork_rs::{EnkaNetwork, Character, NameCard, UserData, IconData, Weapon, Reliquary, block_on, MemoryCache,reqwest::Client};
use image::{DynamicImage, ImageBuffer, Rgba};
use rusttype::{Font, Scale};

#[cfg(feature="redis")]
fn caches()->Result<(MemoryCache,MemoryCache),impl std::fmt::Debug>{
	let client = redis::Client::open("redis://127.0.0.1/")?;
	let rt=tokio::runtime::Builder::new_current_thread().enable_all().build()?;
	let cache=rt.block_on(async move{
		MemoryCache::from(Some(String::from("./cache/")), client).await
	})?;
	Ok::<(MemoryCache, MemoryCache),redis::RedisError>((cache.clone(),cache))
}
#[cfg(not(feature="redis"))]
fn caches()->Result<(MemoryCache,MemoryCache),impl std::fmt::Debug>{
	Ok::<(MemoryCache, MemoryCache),std::io::Error>((MemoryCache::new(String::from("./cache/assets/"))?,
	MemoryCache::new(String::from("./cache/u/"))?))
}
fn main(){
	let start_time=std::time::Instant::now();
	let client=Client::builder().user_agent("ExampleUserAgent").build().ok();
	let (assets_cache,user_cache)=caches().unwrap();
	let mut api=EnkaNetwork::from(client,assets_cache,user_cache);
	let api_copy=api.clone();
	api.set_store(block_on(async move{
		api_copy.store().await.ok()
	}).unwrap());
	let language="ja";
	print_duration("init",&start_time);
	let api_copy=api.clone();
	let font=block_on(async move{
		let url="https://github.com/googlefonts/zen-marugothic/raw/main/fonts/ttf/ZenMaruGothic-Regular.ttf";
		Arc::new(api.web_font(url).await.unwrap())
	}).unwrap();
	//let data=block_on(async move{api_copy.simple(837338702).await}).unwrap().unwrap();
	print_duration("load font data",&start_time);
	enkanetwork_rs::block_on(async move{
		create_user_data(api_copy,618285856,&font,language).await;
		//create_user_data(api_copy,837338702,&font,language).await;
	}).unwrap();
	print_duration("all end",&start_time);
}
async fn create_user_data(api:EnkaNetwork,uid:i32,font:&Font<'static>,language:impl AsRef<str>){
	let data=api.simple(uid).await;
	match data{
		Ok(data)=>{
			std::fs::create_dir_all("./img/").unwrap();
			let profile=data.profile();
			println!("{} AR{} WL{} achievement{}",profile.nickname(),profile.level(),profile.world_level(),profile.achievement());
			println!("{}",profile.signature());
			if let Ok(store)=api.get_store(){
				println!("{:?}",store.locale_list());
			}
			let current:DateTime<Local>=data.lastupdate().into();
			let update:DateTime<Local>=data.reload_time().into();
			println!("current {}", current.format("%Y/%m/%d %T"));
			println!("update {}", update.format("%Y/%m/%d %T"));
			let create_profile_card=async{
				let profile_card=profile_card(&api,&data,&font).await;
				profile_card.save("./img/profile_card.png").unwrap();
			};
			let avatar_list=profile.show_character_list();
			let icons=Arc::new(api.icon_data().await);
			let mut create_char_cards=vec![];
			for id in avatar_list{
				create_char_cards.push(async{
					if let Some(character)=data.character(*id){
						let character_name=character.name(&api,language.as_ref()).unwrap();
						let char_card=character_card(&api,&character,&font,language.as_ref(),&icons).await;
						char_card.save(format!("./img/{}_card.png",&character_name)).unwrap();
					}
				});
			}
			//futures::future::join_all(create_char_cards).await;
			//futures::join!(create_profile_card);
			futures::join!(create_profile_card,futures::future::join_all(create_char_cards));
		},
		Err(e)=>{
			println!("{}",e);
		}
	}
}
async fn profile_card(api:&EnkaNetwork,data:&UserData,font:&Font<'_>)->DynamicImage{
	const NAME_CARD_WIDTH:u32=1260;
	const NAME_CARD_HEIGHT:u32=600;
	let profile=data.profile();
	let show_name_cards=async{
		let mut base=DynamicImage::new_rgba8(NAME_CARD_WIDTH,NAME_CARD_HEIGHT);
		if let Some(img)=show_name_card_list(&api,profile.show_name_card_list()).await{
			image::imageops::overlay(&mut base,&img,0,0);
		};
		base
	};
	let name_card=profile.name_card();
	let name_card=async{
		let name_card=async{
			match name_card.image(&api).await{
				Ok(img)=>{
					img.resize(NAME_CARD_WIDTH,NAME_CARD_HEIGHT,image::imageops::Triangle)
				},
				Err(_)=>DynamicImage::new_rgba8(NAME_CARD_WIDTH,NAME_CARD_HEIGHT)
			}
		};
		let profile_icon=async{
			match data.character(profile.profile_picture()){
				Some(profile_character)=>{
					profile_character.image_icon(api).await
				},
				None=>None
			}
		};
		let (mut name_card,profile_icon)=futures::join!(name_card,profile_icon);
		if let Some(icon)=profile_icon{
			image::imageops::overlay(&mut name_card,&icon,0,0);
		}
		let color_white=image::Rgba([255u8,255u8,255u8,255u8]);
		let scale_15=Scale{x:15f32,y:15f32};
		let text=format!("UID:{}",data.uid());
		imageproc::drawing::draw_text_mut(&mut name_card,color_white,0,0,scale_15,&font,&text);
		let text=format!("{}<{}",profile.nickname(),profile.signature());
		imageproc::drawing::draw_text_mut(&mut name_card,color_white,0,20,scale_15,&font,&text);
		name_card
	};
	let (name_card,show_name_cards)=futures::join!(name_card,show_name_cards);
	let height=if name_card.height()>show_name_cards.height(){
		name_card.height()
	}else{
		show_name_cards.height()
	};
	let mut img=DynamicImage::new_rgba8(name_card.width()+show_name_cards.width(),height);
	image::imageops::overlay(&mut img,&name_card,0,0);
	image::imageops::overlay(&mut img,&show_name_cards,name_card.width() as i64,0);
	img
}
async fn show_name_card_list(api:&EnkaNetwork,name_cards:&Vec<NameCard>)->Option<DynamicImage>{
	let mut name_card_images=vec![];
	let mut width=0;
	for name_card in name_cards{
		if let Ok(name_card)=name_card.image(&api).await{
			let name_card=name_card.resize(420,200,image::imageops::Triangle);
			width=name_card.width();
			name_card_images.push(name_card);
		}
	}
	if !name_card_images.is_empty(){
		if name_card_images.len()>1{
			width*=3;
		}
		let height=(name_card_images.len() as u32/3)*name_card_images.get(0).unwrap().height();
		let mut merge_name_card=DynamicImage::new_rgba8(width,height);
		let mut x=0;
		let mut y=0;
		for name_card in name_card_images{
			image::imageops::overlay(&mut merge_name_card,&name_card,x as i64,y as i64);
			if x<name_card.width()*2{
				x+=name_card.width();
			}else{
				x=0;
				y+=name_card.height();
			}
		}
		Some(merge_name_card)
	}else{
		None
	}
}
async fn character_card(api:&EnkaNetwork,character:&Character,font:&Font<'_>,language:impl AsRef<str>,icons:&IconData)->DynamicImage{
	let mut char_card={
		let char_card=api.assets("img/overlay.jpg").await.unwrap();
		let char_card=std::io::Cursor::new(char_card.as_ref());
		let char_card=image::io::Reader::new(char_card).with_guessed_format().unwrap();
		let char_card=char_card.decode().unwrap();
		let w=char_card.width()-450;
		let h=char_card.height();
		let char_card=image::imageops::crop_imm(&char_card,0,0,w,h).to_image();
		image::imageops::resize(&char_card,
			(char_card.width() as f32*1.2)as u32,(char_card.height() as f32*1.2)as u32,
			image::imageops::Triangle)
	};
	const LIGHT_LEVEL:f32=0.2f32;
	{
		let e=character.element.color_rgb();
		for p in char_card.pixels_mut(){
			let a=p.0[0] as u16+p.0[1] as u16+p.0[2] as u16;
			let mut a=a as f32/765f32+LIGHT_LEVEL;
			if a>1f32{
				a=1f32;
			}
			p.0=[(e[0] as f32*a) as u8,(e[1] as f32*a) as u8,(e[2] as f32*a) as u8,255];
		}
	}
	let avater_image=match character.current_costume(){
		Some(cos)=>cos.image_art(&api).await,
		None=>character.image_gacha_splash(&api).await
	};
	if let Some(avater_image)=avater_image{
		let fit_height=920f32;
		let scale=fit_height/avater_image.height() as f32;
		let w=(avater_image.width() as f32*scale) as u32;
		let h=(avater_image.height() as f32*scale) as u32;
		let mut avater_image=avater_image.resize(w,h,image::imageops::Triangle).into_rgba8();
		let width=avater_image.width();
		let mut x=0;
		let threshold=width/2+300;
		for p in avater_image.pixels_mut(){
			x+=1;
			if x>threshold{
				let alpha=x-threshold;
				let alpha=if alpha>255{
					0
				}else{
					let a=255-alpha as u8;
					if a>p.0[3]{
						p.0[3]
					}else{
						a
					}
				};
				p.0=[p.0[0],p.0[1],p.0[2],alpha];
			}
			if x>=width{
				x=0;
			}
		}
		let y=char_card.height() as i64-avater_image.height() as i64;
		let x=avater_image.width() as i64/2;
		image::imageops::overlay(&mut char_card,&avater_image,400-x,y as i64/2);
	}
	{
		let width=300;
		let mut fade=image::RgbaImage::new(width,char_card.height());
		let mut x_index=0;
		for p in fade.pixels_mut(){
			let alpha=width-x_index;
			let alpha=if alpha>100{
				100
			}else{
				alpha as u8
			};
			p.0=[0,0,0,alpha];
			x_index+=1;
			if x_index>=width{
				x_index=0;
			}
		}
		image::imageops::overlay(&mut char_card,&fade,0,0);
	}
	const CONSTS_SIZE:u32=80;
	let mut images=Vec::with_capacity(character.consts().len());
	for t in character.consts(){
		images.push(async{
			let img=t.image(&api).await.unwrap();
			let mut img=img.into_rgba8();
			let mut base_img=DynamicImage::new_rgba8(CONSTS_SIZE,CONSTS_SIZE).into_rgba8();
			if let Some(c)=icons.image("Const.svg",6f32){
				image::imageops::overlay(&mut base_img,&c,0,0);
			}
			if !t.is_unlock(){
				for p in img.pixels_mut(){
					p.0=[100,100,100,p.0[3]];
				}
			}
			let img=image::imageops::resize(&img,45,45,image::imageops::Triangle);
			image::imageops::overlay(&mut base_img,&img,19,19);
			base_img
		});
	}
	let consts_height=images.len() as u32*(CONSTS_SIZE-5);
	let consts_y=(char_card.height()-consts_height) as i64-20;
	let mut y=0;
	for img in futures::future::join_all(images).await{
		image::imageops::overlay(&mut char_card,&img,0,y as i64+consts_y);
		y+=CONSTS_SIZE-5;
	}
	let color_white=image::Rgba([255u8,255u8,255u8,255u8]);
	{
		let character_name=character.name(&api,&language).unwrap();
		let name_size=50f32;
		imageproc::drawing::draw_text_mut(&mut char_card,color_white,30,20,Scale{x:name_size,y:name_size},&font,&character_name);
		let text=format!("Level {}/{}",character.level,character.ascension_level());
		let text_size=Scale{x:30f32,y:30f32};
		imageproc::drawing::draw_text_mut(&mut char_card,color_white,30,70,text_size,&font,&text);
		let element=character.element.image(&icons,3f32).unwrap();
		image::imageops::overlay(&mut char_card,&element,30,150);
	}
	let friendship_level=character.friendship();
	if friendship_level>0{
		let mut friendship=icons.image_color("Friendship.svg",3f32,image::Rgba([90,90,90,0])).unwrap();
		image::imageops::overlay(&mut char_card,&friendship,30,105);
		for p in friendship.pixels_mut(){
			p.0=[255,255,255,p.0[3]];
		}
		image::imageops::overlay(&mut char_card,&friendship,30,102);
		let text_size=40f32;
		imageproc::drawing::draw_text_mut(&mut char_card,color_white,80,100,Scale{x:text_size,y:text_size},&font,&friendship_level.to_string());
	}
	let status_x={
		let width=char_card.width()/2+200;
		let mut fade=image::RgbaImage::new(width,char_card.height());
		let mut x_index=0;
		for p in fade.pixels_mut(){
			let alpha=if x_index>100{
				100
			}else{
				x_index as u8
			};
			p.0=[0,0,0,alpha];
			x_index+=1;
			if x_index>=width{
				x_index=0;
			}
		}
		let x=(char_card.width()-width) as i64;
		image::imageops::overlay(&mut char_card,&fade,x,0);
		x as i32+90
	};
	let skills=character.skills();
	let mut skill_y=350;
	for skill in skills{
		let img=skill.image(api).await.unwrap();
		let img=img.resize(80,80,image::imageops::Triangle);
		image::imageops::overlay(&mut char_card,&img,status_x as i64,skill_y as i64);
		let text=format!("{}",skill.level()+skill.extra_level());
		let text_size=Scale{x:30f32,y:30f32};
		let (tw,th)=imageproc::drawing::text_size(text_size,&font,&text);
		imageproc::drawing::draw_text_mut(&mut char_card,color_white,status_x+40-tw/2,skill_y+th+50,text_size,&font,&text);
		skill_y+=100;
	}
	let weapon=character.weapon();
	render_weapon(api,&mut char_card,weapon,&language,icons,font,status_x+120,30).await;
	let set_name=render_reliquarys(api,&mut char_card,character.reliquarys(),icons,font,status_x+530,20).await;
	fn get_set_name(api: &EnkaNetwork,language: impl AsRef<str>,hash:u32)->Option<&str>{
		api.get_store().ok()?.locale(language,hash.to_string())
	}
	let text_size=Scale{x:30f32,y:30f32};
	let mut y=630;
	for (k,v) in set_name{
		if v<2{
			continue;
		}
		if let Some(s)=get_set_name(api,&language,k){
			imageproc::drawing::draw_text_mut(&mut char_card,color_white,status_x+150,y,text_size,&font,s);
		}
		let set=if v>2{
			"4"
		}else{
			"2"
		};
		imageproc::drawing::draw_text_mut(&mut char_card,color_white,status_x+500,y,text_size,&font,set);
		y+=40;
	}
	render_fight_prop(api,&mut char_card,character,&language,icons,font,status_x+120,150);
	image::DynamicImage::ImageRgba8(char_card)
}
async fn render_reliquarys(
	api:&EnkaNetwork,
	char_card:&mut image::RgbaImage,
	reliquarys:&Vec<Reliquary>,
	icons:&IconData,
	font:&Font<'_>,
	status_x:i32,
	status_y:i32)->HashMap<u32,u8>{
	let color_white=image::Rgba([255u8,255u8,255u8,255u8]);
	let mut reliquary_images=Vec::with_capacity(reliquarys.len());
	for r in reliquarys{
		reliquary_images.push(r.image_icon(api));
	}
	let reliquary_images=futures::future::join_all(reliquary_images).await;
	let height=130;
	let bg_img={
		let width=450;
		let mut img:ImageBuffer<Rgba<u8>,Vec<u8>>=image::ImageBuffer::new(width,height);
		let mut i=0;
		for px in img.pixels_mut(){
			let alpha=if i<160{
				i as u8
			}else{
				160
			};
			i+=1;
			px.0=[0,0,0,alpha];
			if i>=width{
				i=0;
			}
		}
		img
	};
	let mut set_name=HashMap::new();
	for i in 0..reliquarys.len(){
		let layout_y=status_y+i as i32*(height as i32+10);
		let r=reliquarys.get(i).unwrap();
		let set_name_hash=r.set_name_hash();
		set_name.insert(set_name_hash,*set_name.get(&set_name_hash).unwrap_or(&0u8)+1);
		let img=reliquary_images.get(i).unwrap();
		match img{
			Ok(img)=>{
				let img=img.resize(height,height,image::imageops::Triangle);
				image::imageops::overlay(char_card,&img,status_x as i64,layout_y as i64);
			},Err(e)=>println!("{}",e)
		}
		image::imageops::overlay(char_card,&bg_img,status_x as i64,layout_y as i64);
		if let Some(mut img)=r.main_stats.0.image(icons,2f32){
			if let enkanetwork_rs::Stats::ElementAddHurt(_)=r.main_stats.0{

			}else{
				for p in img.pixels_mut(){
					p.0=[255,255,255,p.0[3]];
				}
			}
			image::imageops::overlay(char_card,&img,status_x as i64+100,layout_y as i64+40);
		}
		let text=r.main_stats.to_string();
		let text_size=Scale{x:50f32,y:50f32};
		let (tw,_)=imageproc::drawing::text_size(text_size,&font,&text);
		imageproc::drawing::draw_text_mut(char_card,color_white,status_x+140-tw,layout_y+60,text_size,&font,&text);
		let text=format!("☆{} +{}",r.rarity,r.level);
		let text_size=Scale{x:20f32,y:20f32};
		let (tw,_)=imageproc::drawing::text_size(text_size,&font,&text);
		imageproc::drawing::draw_text_mut(char_card,color_white,status_x+140-tw,layout_y+100,text_size,&font,&text);
		
		let text_size=Scale{x:40f32,y:40f32};
		let layout=[
			(status_x+160,layout_y+20),
			(status_x+300,layout_y+20),
			(status_x+160,layout_y+80),
			(status_x+300,layout_y+80)];
		for i in 0..r.sub_stats.len(){
			if let Some(s)=r.sub_stats[i]{
				imageproc::drawing::draw_text_mut(char_card,color_white,layout[i].0+35,layout[i].1-10,text_size,&font,&s.to_string());
				if let Some(mut img)=s.0.image(icons,2f32){
					for p in img.pixels_mut(){p.0=[255,255,255,p.0[3]]}
					image::imageops::overlay(char_card,&img,layout[i].0 as i64,layout[i].1 as i64);
				}
			}
		}
	}
	set_name
}
async fn render_weapon(
	api:&EnkaNetwork,
	char_card:&mut image::RgbaImage,
	weapon:&Weapon,
	language:impl AsRef<str>,
	icons:&IconData,
	font:&Font<'_>,
	status_x:i32,
	status_y:i32){
	let color_white=image::Rgba([255u8,255u8,255u8,255u8]);
	if let Ok(weapon_icon)=weapon.image_icon(api).await{
		let weapon_icon=weapon_icon.resize(130,130,image::imageops::Triangle);
		image::imageops::overlay(char_card,&weapon_icon,status_x as i64,status_y as i64);
	}
	if let Some(text)=weapon.name(api,&language){
		let text_size=Scale{x:30f32,y:30f32};
		imageproc::drawing::draw_text_mut(char_card,color_white,status_x+180,status_y,text_size,&font,&text);
	}
	{
		let text_size=Scale{x:30f32,y:30f32};
		let mut atk_icon=enkanetwork_rs::Stats::Attack.image(icons,1.5f32).unwrap();
		for p in atk_icon.pixels_mut(){
			p.0=[255,255,255,p.0[3]];
		}
		image::imageops::overlay(char_card,&atk_icon,(status_x+180) as i64,(status_y+45) as i64);
		let text=format!("{}",weapon.base_attack);
		imageproc::drawing::draw_text_mut(char_card,color_white,status_x+210,status_y+40,text_size,&font,&text);
	}
	if let Some(stats)=weapon.stats{
		let text_size=Scale{x:30f32,y:30f32};
		let mut stats_icon=stats.0.image(icons,1.5f32).unwrap();
		for p in stats_icon.pixels_mut(){
			p.0=[255,255,255,p.0[3]];
		}
		image::imageops::overlay(char_card,&stats_icon,(status_x+250) as i64,(status_y+45) as i64);
		let text=format!("{}",stats.1);
		imageproc::drawing::draw_text_mut(char_card,color_white,status_x+280,status_y+40,text_size,&font,&text);
	}
	{
		let text_size=Scale{x:30f32,y:30f32};
		let text=format!("R{}",weapon.refinement+1);
		imageproc::drawing::draw_text_mut(char_card,color_white,status_x+180,status_y+80,text_size,&font,&text);
		let text=format!("Level {}/{}",weapon.level,weapon.ascension_level());
		imageproc::drawing::draw_text_mut(char_card,color_white,status_x+210,status_y+80,text_size,&font,&text);
	}
}
fn render_fight_prop(api:&EnkaNetwork,char_card:&mut image::RgbaImage,character:&Character
	,language:impl AsRef<str>,icons:&IconData,font:&Font,fight_prop_x:i32,fight_prop_y:i32){
	let mut fight_prop_y=fight_prop_y;
	let pad=400;
	let text_size=Scale{x:40f32,y:40f32};
	let subvalue_size=Scale{x:30f32,y:30f32};
	let color_white=image::Rgba([255u8,255u8,255u8,255u8]);
	let fploc=api.fight_prop_locale(&language).unwrap();
	let fight=character.fight_prop();

	fight_prop_y+=50;
	imageproc::drawing::draw_text_mut(char_card,color_white,fight_prop_x,fight_prop_y,text_size,&font,fploc.max_hp);
	let text=format!("{}",fight.display_max_hp.round() as i32);
	let (x,_)=imageproc::drawing::text_size(text_size,&font,&text);
	imageproc::drawing::draw_text_mut(char_card,color_white,fight_prop_x+pad-x,fight_prop_y,text_size,&font,&text);
	fight_prop_y+=5;
	let text=format!("{}+{}",fight.base_hp.round() as i32,(fight.display_max_hp-fight.base_hp).round() as i32);
	let (x,y)=imageproc::drawing::text_size(subvalue_size,&font,&text);
	imageproc::drawing::draw_text_mut(char_card,color_white,fight_prop_x+pad-x,fight_prop_y+y,subvalue_size,&font,&text);
	if let Some(icon_image)=icons.image_color("FIGHT_PROP_HP.svg",1.5f32,color_white){
		image::imageops::overlay(char_card,&icon_image,fight_prop_x as i64-30i64,fight_prop_y as i64+8i64);
	}

	fight_prop_y+=50;
	imageproc::drawing::draw_text_mut(char_card,color_white,fight_prop_x,fight_prop_y,text_size,&font,fploc.attack);
	let text=format!("{}",fight.display_attack.round() as i32);
	let (x,_)=imageproc::drawing::text_size(text_size,&font,&text);
	imageproc::drawing::draw_text_mut(char_card,color_white,fight_prop_x+pad-x,fight_prop_y,text_size,&font,&text);
	fight_prop_y+=5;
	let text=format!("{}+{}",fight.base_attack.round() as i32,(fight.display_attack-fight.base_attack).round() as i32);
	let (x,y)=imageproc::drawing::text_size(subvalue_size,&font,&text);
	imageproc::drawing::draw_text_mut(char_card,color_white,fight_prop_x+pad-x,fight_prop_y+y,subvalue_size,&font,&text);
	if let Some(icon_image)=icons.image_color("FIGHT_PROP_ATTACK.svg",1.5f32,color_white){
		image::imageops::overlay(char_card,&icon_image,fight_prop_x as i64-30i64,fight_prop_y as i64+8i64);
	}

	fight_prop_y+=50;
	imageproc::drawing::draw_text_mut(char_card,color_white,fight_prop_x,fight_prop_y,text_size,&font,fploc.defense);
	let text=format!("{}",fight.display_defense.round() as i32);
	let (x,_)=imageproc::drawing::text_size(text_size,&font,&text);
	imageproc::drawing::draw_text_mut(char_card,color_white,fight_prop_x+pad-x,fight_prop_y,text_size,&font,&text);
	fight_prop_y+=5;
	let text=format!("{}+{}",fight.base_defense.round() as i32,(fight.display_defense-fight.base_defense).round() as i32);
	let (x,y)=imageproc::drawing::text_size(subvalue_size,&font,&text);
	imageproc::drawing::draw_text_mut(char_card,color_white,fight_prop_x+pad-x,fight_prop_y+y,subvalue_size,&font,&text);
	if let Some(icon_image)=icons.image_color("FIGHT_PROP_DEFENSE.svg",1.5f32,color_white){
		image::imageops::overlay(char_card,&icon_image,fight_prop_x as i64-30i64,fight_prop_y as i64+8i64);
	}

	fight_prop_y+=50;
	imageproc::drawing::draw_text_mut(char_card,color_white,fight_prop_x,fight_prop_y,text_size,&font,fploc.elemental_mastery);
	let text=format!("{}",fight.elemental_mastery.round() as i32);
	let (x,_)=imageproc::drawing::text_size(text_size,&font,&text);
	imageproc::drawing::draw_text_mut(char_card,color_white,fight_prop_x+pad-x,fight_prop_y,text_size,&font,&text);
	if let Some(icon_image)=icons.image_color("FIGHT_PROP_ELEMENT_MASTERY.svg",1.5f32,color_white){
		image::imageops::overlay(char_card,&icon_image,fight_prop_x as i64-30i64,fight_prop_y as i64+13i64);
	}

	fight_prop_y+=50;
	imageproc::drawing::draw_text_mut(char_card,color_white,fight_prop_x,fight_prop_y,text_size,&font,fploc.critical_rate);
	let text=format!("{:.*}%",1,fight.critical_rate*100f64);
	let (x,_)=imageproc::drawing::text_size(text_size,&font,&text);
	imageproc::drawing::draw_text_mut(char_card,color_white,fight_prop_x+pad-x,fight_prop_y,text_size,&font,&text);
	if let Some(icon_image)=icons.image_color("FIGHT_PROP_CRITICAL.svg",1.5f32,color_white){
		image::imageops::overlay(char_card,&icon_image,fight_prop_x as i64-30i64,fight_prop_y as i64+13i64);
	}

	fight_prop_y+=50;
	imageproc::drawing::draw_text_mut(char_card,color_white,fight_prop_x,fight_prop_y,text_size,&font,fploc.critical_damage);
	let text=format!("{:.*}%",1,fight.critical_damage*100f64);
	let (x,_)=imageproc::drawing::text_size(text_size,&font,&text);
	imageproc::drawing::draw_text_mut(char_card,color_white,fight_prop_x+pad-x,fight_prop_y,text_size,&font,&text);
	if let Some(icon_image)=icons.image_color("FIGHT_PROP_CRITICAL_HURT.svg",1.5f32,color_white){
		image::imageops::overlay(char_card,&icon_image,fight_prop_x as i64-30i64,fight_prop_y as i64+13i64);
	}

	fight_prop_y+=50;
	imageproc::drawing::draw_text_mut(char_card,color_white,fight_prop_x,fight_prop_y,text_size,&font,fploc.energy_recharge);
	let text=format!("{:.*}%",1,fight.energy_recharge*100f64);
	let (x,_)=imageproc::drawing::text_size(text_size,&font,&text);
	imageproc::drawing::draw_text_mut(char_card,color_white,fight_prop_x+pad-x,fight_prop_y,text_size,&font,&text);
	if let Some(icon_image)=icons.image_color("FIGHT_PROP_CHARGE_EFFICIENCY.svg",1.5f32,color_white){
		image::imageops::overlay(char_card,&icon_image,fight_prop_x as i64-30i64,fight_prop_y as i64+13i64);
	}

	for (element,dmg) in &fight.damage_bonus{
		fight_prop_y+=50;
		let text=match api.get_store(){
			Ok(store)=>element.attack_name(store, language),_=>""
		};
		imageproc::drawing::draw_text_mut(char_card,color_white,fight_prop_x,fight_prop_y,text_size,&font,&text);
		let text=format!("{:.*}%",1,dmg*100f64);
		let (x,_)=imageproc::drawing::text_size(text_size,&font,&text);
		imageproc::drawing::draw_text_mut(char_card,color_white,fight_prop_x+pad-x,fight_prop_y,text_size,&font,&text);
		if let Some(icon_image)=element.image_color(icons,1.5f32,color_white){
			image::imageops::overlay(char_card,&icon_image,fight_prop_x as i64-30i64,fight_prop_y as i64+13i64);
		}
		break;//limit line????????
	}
}
fn print_duration(s:&str,start_time:&std::time::Instant){
	let now=std::time::Instant::now();
	let time=now-*start_time;
	println!("{} {}ms",s,time.as_millis());
}
