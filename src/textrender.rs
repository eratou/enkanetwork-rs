use std::borrow::Cow;

use rusttype::Font;

use crate::EnkaNetwork;

impl EnkaNetwork{
	pub async fn web_font(&self,url:impl AsRef<str>)->Result<Font<'static>,String>{
		let font_file=self.assets(url).await?;
		load_font(Cow::Borrowed(font_file.as_ref()))
	}
}
pub fn load_font(v:Cow<Vec<u8>>)->Result<Font<'static>,String>{
	let ttf=if woff2::decode::is_woff2(&v){
		match woff2::decode::convert_woff2_to_ttf(&mut std::io::Cursor::new(v.as_ref())){
			Ok(v)=>v,
			Err(e)=>return Err(format!("woff2 decode {}",e))
		}
	}else{
		if v.starts_with(br"wOFF"){
			let mut otf=std::io::Cursor::new(vec![]);
			match woff::convert_woff_to_otf(&mut std::io::Cursor::new(v.as_ref()),&mut otf){
				Ok(_)=>otf.into_inner(),
				Err(_)=>return Err(String::from("woff decode error"))
			}
		}else{
			match v{
				Cow::Borrowed(b)=>b.to_vec(),
				Cow::Owned(o)=>o
			}
		}
	};
	Ok(match Font::try_from_vec(ttf){
		Some(s)=>s,
		None=>return Err(String::from("ttf decode"))
	})
}
