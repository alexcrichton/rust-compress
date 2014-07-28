#![crate_type = "bin"]
#![deny(warnings, missing_doc)]
#![feature(macro_rules, phase)]

//! A very simple version of "gunzip", for testing.

#[phase(plugin, link)] extern crate log;
extern crate compress;

use std::{os,io};
use std::io::{IoResult,File};
use std::vec::Vec;
use std::path::Path;
use compress::gzip;
use compress::checksum::crc;

fn main(){
    let mut files : Vec<Path> = Vec::new();
    for name in os::args().slice_from(1).iter() {
        let path = Path::new(name.clone());
        if path.is_file() && path.extension_str() == Some("gz"){
            files.push(path);
        }
    }
    
    if files.len() == 0 {
        println!("Usage: {} FILE1.gz [FILE2.gz ...]", os::args().get(0));
        std::os::set_exit_status(1);
        return;
    }
    
    match run(files) {
        Ok(()) => {},
        Err(msg) => {
            println!("Error: {}", msg);
            std::os::set_exit_status(1);
        }
    }
}

fn run(files: Vec<Path>) -> IoResult<()> {
    let crc_table = crc::Table32::new();
    for file in files.iter() {
        println!("Reading file {}", file.display());
        let stream = File::open(file);
        let mut decoder = gzip::Decoder::new(stream, &crc_table);
        loop {
            match decoder.member() {
                Ok(ref mut memb) => {
                    if memb.file_name.len() > 0 {
                        println!("Member: {}", memb.file_name);
                    } else {
                        println!("Member: no name");
                    }
                    if memb.file_comment.len() > 0 {
                        println!("Comment: {}", memb.file_comment);
                    }
                    let content = try!(memb.read_to_end());
                    if memb.file_name.len() > 0 {
                        let path = Path::new(memb.file_name.as_slice());
                        if path.exists() {
                            return Err(io::IoError {
                                kind: io::PathAlreadyExists,
                                desc: "file already exists",
                                detail: Some(memb.file_name.to_str())
                            });
                        }
                        let mut out_f = try!(File::create(&path));
                        try!(out_f.write(content.as_slice()));
                    }else{
                        println!("================\n{}================", content);
                    }
                },
                Err(ref e) if e.kind == io::EndOfFile => {
                    break; // we're done for this file
                },
                Err(e) => {
                    return Err(e);    // stop now
                }
            }
        }
    }
    Ok(())
}
