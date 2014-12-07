extern crate toml;
extern crate glob;

use std::io::fs::PathExtensions;
use std::io::fs;
use std::io;
use std::os;

use app_result::{AppResult, app_err};
use dependencies::read_dependencies;
use types::TagsRoot;

use tags::{
   update_tags,
   update_tags_and_check_for_reexports,
   create_tags,
   merge_tags
};

use dirs::rusty_tags_dir;

mod app_result;
mod dependencies;
mod dirs;
mod tags;
mod types;

fn main() 
{
   update_all_tags().unwrap_or_else(|err| {
      let stderr = &mut io::stderr();
      let _ = writeln!(stderr, "rusty-tags: {}", err);
      os::set_exit_status(1);
   });
}

fn update_all_tags() -> AppResult<()>
{
   let cwd = try!(os::getcwd());
   let cargo_dir = try!(find_cargo_toml_dir(&cwd));
   let tags_roots = try!(read_dependencies(&cargo_dir));

   for tags_root in tags_roots.iter() {
      let mut tag_files: Vec<Path> = Vec::new();
      let mut tag_dir: Option<Path> = None;

      match *tags_root {
         TagsRoot::Src { ref src_dir, ref dependencies } => {
            let mut src_tags = src_dir.clone();
            src_tags.push(rusty_tags_file_name());
            try!(create_tags(src_dir, &src_tags));
            tag_files.push(src_tags);

            for dep in dependencies.iter() {
               tag_files.push(try!(update_tags(dep)).tags_file);
            }

            tag_dir = Some(src_dir.clone());
         },

         TagsRoot::Lib { ref src_kind, ref dependencies } => {
            let lib_tags = try!(update_tags_and_check_for_reexports(src_kind, dependencies));
            if lib_tags.cached {
               let mut src_tags = lib_tags.src_dir.clone();
               src_tags.push(rusty_tags_file_name());
               if src_tags.is_file() {
                  continue;
               }
            }

            tag_files.push(lib_tags.tags_file);

            for dep in dependencies.iter() {
               tag_files.push(try!(update_tags(dep)).tags_file);
            }

            tag_dir = Some(lib_tags.src_dir.clone());
         }
      }

      if tag_files.is_empty() || tag_dir.is_none() {
         continue;
      }

      let mut rust_tags = try!(rusty_tags_dir());
      rust_tags.push("rust");
      if rust_tags.is_file() {
         tag_files.push(rust_tags);
      }

      let mut tags_file = tag_dir.unwrap();
      tags_file.push(rusty_tags_file_name());

      try!(merge_tags(&tag_files, &tags_file));
   }

   Ok(())
}

/// Searches for a directory containing a `Cargo.toml` file starting at
/// `start_dir` and continuing the search upwards the directory tree
/// until a directory is found.
fn find_cargo_toml_dir(start_dir: &Path) -> AppResult<Path>
{
   let mut dir = start_dir.clone();
   loop {
      if let Ok(files) = fs::readdir(&dir) {
         for file in files.iter() {
            if file.is_file() {
               if let Some("Cargo.toml") = file.filename_str() {
                  return Ok(dir);
               }
            }
         }
      }

      if ! dir.pop() {
         return Err(app_err(format!("Couldn't find 'Cargo.toml' starting at directory '{}'!", start_dir.display())));
      }
   }
}

/// the name under which the tags files are saved
fn rusty_tags_file_name() -> &'static str
{
   "rusty.tags"
}
