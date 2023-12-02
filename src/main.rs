use std::{io, fs};
use std::path;
fn from_album_source<'s>(source: &'s str) -> impl Iterator<Item=(&'s str, &'s str)> {
    source.lines().filter_map(|x| x.split_once('='))
}

#[derive(Debug)]
struct Album {
    title: String,
    songs: Vec<(String, String)>,
}

fn load_album<P: AsRef<path::Path> + ?Sized>(source: &P) -> io::Result<Album> {
    let path = source.as_ref();
    path.is_file();
    let filename = path.file_name().unwrap().to_str().unwrap(); 
    let source = fs::read_to_string(path)?;
    let songs = from_album_source(&source).map(|(a,b)|(a.to_string(), b.to_string())).collect::<Vec<_>>();
    Ok(Album{title: filename.to_string(), songs})
}

fn main() -> io::Result<()>{
    let album = load_album("./res/test.album")?;
    println!("{album:?}");
    Ok(())
}
