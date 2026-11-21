use std::{fs::File,io::{BufReader,Read,Seek},time::Instant};
fn probe<R:Read+Seek>(r:R)->usize{
 let mut a=zip::ZipArchive::new(r).unwrap();
 assert!(matches!(a.by_name("korrust/probe/Missing.class"),Err(zip::result::ZipError::FileNotFound)));
 a.len()
}
fn main(){
 let args:Vec<_>=std::env::args().collect(); let start=Instant::now();let mut count=0;
 for _ in 0..args[3].parse::<usize>().unwrap(){
  let file=File::open(&args[2]).unwrap();
  count+=match args[1].as_str(){"raw"=>probe(file),"buffered"=>probe(BufReader::new(file)),_=>panic!()};
 }
 println!("{} {} entries elapsed_ns {}",args[1],count,start.elapsed().as_nanos());
}
