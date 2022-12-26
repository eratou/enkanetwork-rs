use std::collections::HashMap;

use image::{RgbaImage, ImageBuffer};
use resvg::tiny_skia::{PixmapMut, Transform};
use usvg::FitTo;

use crate::{EnkaNetwork, Element, Stats};

const ICON_ZIP_URL:&str="https://cdn.discordapp.com/attachments/819643218446254100/960919577867485204/icon.zip";

pub struct IconData(HashMap<String,Vec<u8>>);
impl EnkaNetwork{
	pub async fn icon_data(&self)->IconData{
		IconData::load(self).await
	}
}
impl IconData{
	pub async fn load(api:&EnkaNetwork)->Self{
		let icon_zip=api.assets(ICON_ZIP_URL).await.unwrap();
		let icon_zip=icon_zip.as_ref();
		let mut zip_archive=zip::ZipArchive::new(std::io::Cursor::new(icon_zip)).unwrap();
		let mut datas=HashMap::new();
		for i in 0..zip_archive.len(){
			let mut contents=zip_archive.by_index(i).unwrap();
			if contents.is_file(){
				let mut writer:Vec<u8>=vec![];
				std::io::copy(&mut contents,&mut writer).unwrap();
				let name=contents.name().to_owned();
				datas.insert(name,writer);
			}
		}
		Self(datas)
	}
	pub fn svg(&self,path:impl AsRef<str>)->Option<&Vec<u8>>{
		self.0.get(path.as_ref())
	}
	pub fn image(&self,path:impl AsRef<str>,zoom:f32)->Option<RgbaImage>{
		let bytes=self.svg(path)?;
		let ops=usvg::Options::default();
		//ops.fontdb.load_system_fonts();
		//ops.fontdb.load_font_data(bytes.to_owned());
		let tree=usvg::Tree::from_data(&bytes,&ops).ok()?;
		let size=tree.size;
		let fit=FitTo::Zoom(zoom);
		let tf=Transform::identity();
		let width=size.width()*zoom as f64;
		let height=size.height()*zoom as f64;
		let mut rgba8=vec![0;(width as usize*height as usize*4) as usize];
		let pxmap=PixmapMut::from_bytes(&mut rgba8,width as u32,height as u32)?;
		resvg::render(&tree,fit,tf,pxmap);
		let img:RgbaImage=ImageBuffer::from_raw(width as u32,height as u32,rgba8)?;
		Some(img)
	}
	pub fn image_color(&self,path:impl AsRef<str>,zoom:f32,color:image::Rgba<u8>)->Option<RgbaImage>{
		let mut img=self.image(path,zoom)?;
		for px in img.pixels_mut(){
			px.0=[color.0[0],color.0[1],color.0[2],px.0[3]];
		}
		Some(img)
	}
}
impl Element{
	pub fn image(&self,data:&IconData,zoom:f32)->Option<RgbaImage>{
		data.image(format!("FIGHT_PROP_{}_ADD_HURT.svg",self.fight_prop_name()),zoom)
	}
	pub fn image_color(&self,data:&IconData,zoom:f32,color:image::Rgba<u8>)->Option<RgbaImage>{
		data.image_color(format!("FIGHT_PROP_{}_ADD_HURT.svg",self.fight_prop_name()),zoom,color)
	}
}
impl Stats{
	pub fn image(&self,data:&IconData,zoom:f32)->Option<RgbaImage>{
		data.image(format!("{}.svg",self.id()),zoom)
	}
}
