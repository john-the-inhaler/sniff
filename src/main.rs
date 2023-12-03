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
    // `Ok(_)` means add, `Err(_)` means delete
    fn diff<'a>(&'a self, oth: &'a Self) -> (Vec<(&'a str, &'a str)>, Vec<&'a str>) {
       // using twoside shuffling
       let mut additions = Vec::new();
       let mut deletions = Vec::new();
       // Everything in here and not in there needs to be added!
       for (k, v) in self.songs.iter() {
           if let Some(v2) = oth.contains(k) {
               if v2 == v { continue; }
           }
           additions.push((&k[..], &v[..]));
       }
       // everything in there not in here is to be removed
       for (k, v) in oth.songs.iter() {
           if let Some(v2) = self.contains(k) {
               if v2 == v { continue; }
           }
           deletions.push(&k[..]);
       }
       (additions, deletions)
    }
    fn render(&self) -> String {
        // calculate size
        let mut size = 0;
        for (key, value) in self.songs.iter() {
            size += key.len() + 1 + value.len() + 1;
        }
        let mut buff = String::with_capacity(size);
        for (key, value) in self.songs.iter() {
            buff.push_str(&key);
            buff.push('=');
            buff.push_str(&value);
            buff.push('\n');
        }
        buff        
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
           .stdout(process::Stdio::null()) 
           .stderr(process::Stdio::null());
    println!("[{ident}] invoking:\n{command:?}");
    let mut child = command.spawn()?;
    let result = child.wait()?;
    if !result.success() {
        println!("[{ident}] Audio Extraction failed");
        return Ok(false);
    }
    println!("[{ident}] Audio Extraction success");
    println!("[{ident}] Cleaning up temp files");
    fs::remove_file(temp_file)?;
    Ok(true) 
}
fn detach<'a, 'b: 'a, T: ?Sized>(x: &'a T) -> &'b T {
    use std::ptr::{read, addr_of};
    unsafe{ read(addr_of!(x).cast()) }
}
fn enact_plan(album: &Album, strategy: Strategy) -> io::Result<()> {
    println!("[{}] generating plan", &album.title);
    let mut additions = Vec::with_capacity(0);
    let mut deletions = Vec::with_capacity(0);
    match strategy {
        Strategy::New => { 
            additions.extend(album.songs.iter().map(|(a, b)|(a.as_str(), b.as_str())));
        }
        Strategy::ReBuild(other) => {
            let (addit, delet) = album.diff(detach(&other));
            for i in addit {additions.push(i);}
            for i in delet {deletions.push(i);}
        }
    }
    let mut path = env::current_dir()?;
    path.push(&album.title);
    if !path.exists() {
        println!("[{}] album folder not found, making it", &album.title);
        fs::create_dir(&path)?;
    }
    println!("[{}] performing deletions", &album.title);
    for target in deletions {
        path.push(target);
        path.set_extension("mp3");
        println!("[{}] deleting `{}`", &album.title, path.display());
        fs::remove_file(&path)?;
        path.pop();
    }
    println!("[{}] performing additions", &album.title);
    for (target, id) in additions {
        path.push(target);
        path.set_extension("mp3");
        println!("[{}] downloading `{}`", &album.title, path.display());
        download_song(path.to_str().unwrap(), id)?;
        path.pop();
    }
    println!("[{}] creating `.album` file", &album.title);
    path.push(".album");
    //render album to string
    let mani = album.render();
    fs::write(&path, mani)?;
    Ok(())
}

fn main() -> io::Result<()>{
    // creating the .sniff directory
    let mut sniff = env::current_dir()?;
    sniff.push(".sniff");
    if !sniff.exists() {
        println!("`.sniff` not found, creating `.sniff`");
        fs::create_dir(sniff)?;
    }
    
    let album = load_album_file(&env::args().skip(1).next().unwrap())?.expect("fuck this");
    println!("{album:?}");

    let strategy = get_strategy(&album)?;
    enact_plan(&album, strategy)?;


    Ok(())
}
