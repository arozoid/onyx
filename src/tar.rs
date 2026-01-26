use std::fs::File;
use std::io;
use tar::Archive;
use zstd::stream::read::Decoder;

pub fn extract_tar_zst(path: &str, dest: &str) -> io::Result<()> {
    // open the .tar.zst file
    let tar_zst = File::open(path)?;

    // create a streaming decoder for zstd
    let decoder = Decoder::new(tar_zst)?;  

    // create a tar archive reader
    let mut archive = Archive::new(decoder);

    // extract all files to destination
    archive.unpack(dest)?;  

    Ok(())
}