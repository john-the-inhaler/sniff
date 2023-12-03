use std::{io::{self, BufRead}, fs};
use std::env;
use std::path;

fn from_album_source<'s>(source: &'s str) -> impl Iterator<Item=(&'s str, &'s str)> {
    source.lines().filter_map(|x| x.split_once('='))
}

#[derive(Debug)]
struct Album {
    title: String,
    songs: Vec<(String, String)>,
}

impl Album {
    fn same_ident(&self, oth: &Self) -> bool {
        return self.title == oth.title;
    }
    fn contains(&self, key: &str) -> Option<&str> {
        for (k, v) in self.songs.iter() {
            if k == key { return Some(v); }
        }
        return None;
    }
    fn diff<'a>(&'a self, oth: &'a Self) -> Vec<(&'a str, &'a str)> {
       // using twoside shuffling
       let mut result = Vec::new();
       for (k, v) in self.songs.iter() {
           if let Some(v2) = oth.contains(k) {
               if v2 == v { continue; }         
           }
           result.push((&k[..], &v[..]));
       }
       for (k, v) in oth.songs.iter() {
           if let Some(v2) = self.contains(k) {
               if v2 == v { continue; }         
           }
           result.push((&k[..], &v[..]));
       }
       result
    }
}

fn load_album_file<P: AsRef<path::Path> + ?Sized>(source: &P) -> io::Result<Option<Album>> {
    let path = source.as_ref();
    if !path.is_file() {return Ok(None);}
    let filename = path.file_name().unwrap().to_str().unwrap(); 
    let filename = filename.split_once('.').map(|x|x.0).unwrap_or(filename);
    let source = fs::read_to_string(path)?;
    let songs = from_album_source(&source).map(|(a,b)|(a.to_string(), b.to_string())).collect::<Vec<_>>();
    Ok(Some(Album{title: filename.to_string(), songs}))
}

#[derive(Debug, Clone, Copy)]
enum FLFR { NotFile, NotFolder }
fn load_album_folder<P: AsRef<path::Path> + ?Sized>(source: &P) -> io::Result<Result<Album, FLFR>> {
    let mut path = source.as_ref().to_owned();
    if !path.is_dir() { return Ok(Err(FLFR::NotFolder)); }
    path.push(".album");
    if !path.is_file() { return Ok(Err(FLFR::NotFile)); }
    let contents = fs::read_to_string(&path)?;
    let songs = from_album_source(&contents).map(|(a,b)|(a.to_string(), b.to_string())).collect::<Vec<_>>();
    path.pop();
    let filename = path.file_name().unwrap().to_str().unwrap();
    Ok(Ok(Album{title: filename.to_string(), songs}))
}
enum Strategy {
    New, // When the album doesn't exist
    ReBuild(Album) // The album exists
}
fn get_strategy(album: &Album) -> io::Result<Strategy> {
    let mut path = env::current_dir()?;
    path.push(&album.title[..]);
    if !path.exists() {
        return Ok(Strategy::New);
    }
    let pre = load_album_folder(&path)?;
    if pre.is_err() {
        println!("invalid album {path:?}, treating as empty");
        return Ok(Strategy::New);
    }
    Ok(Strategy::ReBuild(pre.unwrap())) 
}

use std::process::{self, Command};
fn download_song(dest: &str, ident: &str) -> io::Result<bool> {
    use io::Read;
    println!("[{ident}] Downloading");
    let mut command = Command::new("yt-dlp");
    command.arg("--print")
           .arg("after_move:filepath")
           .arg("-o")
           .arg("./.sniff/%(id)s.%(ext)s")
           .arg(ident)
           .stdout(process::Stdio::piped());
    println!("[{ident}] invoking:\n{command:?}");
    let mut child = command.spawn()?;
    let result = child.wait()?;
    if !result.success() {
        println!("[{ident}] Download failed");
        return Ok(false);
    }
    println!("[{ident}] Download Success");
    let mut temp_file = io::BufReader::new(child.stdout.unwrap()).lines().next().unwrap()?;
    println!("[{ident}] source video downloaded to `{temp_file}`");
    println!("[{ident}] Extracting audio");
    
    let mut command = Command::new("ffmpeg");
    command.arg("-i")
           .arg(&temp_file[..])
           .arg(dest)
           .stdout(process::Stdio::piped());
    println!("[{ident}] invoking:\n{command:?}");
    let mut child = command.spawn()?;
    let result = child.wait()?;
    if !result.success() {
        println!("[{ident}] Audio Extraction failed");
        return Ok(false);
    }
    println!("[{ident}] Audio Extraction success");
    println!("[{ident}] Cleaning up temp files");
    Command::new("rm").arg(temp_file).spawn()?;
    Ok(true) 
}

fn main() -> io::Result<()>{
    // creating the .sniff directory
    let mut sniff = env::current_dir()?;
    sniff.push(".sniff");
    if !sniff.exists() {
        println!("`.sniff` not found, creating `.sniff`");
        fs::create_dir(sniff)?;
    }
    
    let album = load_album_file("./res/test.album")?.expect("fuck this");
    println!("{album:?}");

    let test_dest = "./.sniff/bop.mp3";

    download_song(test_dest, &album.songs[0].1)?;

    Ok(())
}
