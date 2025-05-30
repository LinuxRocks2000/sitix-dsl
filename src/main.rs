mod utility;
mod lexer;
mod ast;
mod parse;
mod inflate;
mod interpret;
use interpret::*;
mod resolve;
mod ffi;
use ffi::*;
mod error;
mod filesystem;
use clap::{ Parser, Subcommand };
use std::path::PathBuf;
use crate::resolve::*;
use inotify::EventMask;


// parsing works in three stages.
/*
   Stage 1: Lexer
    A knight travels through the land, accumulating much dust about him; it takes but an LL(1) interpreting step to scrub clean
    the shining armor within!

    Code is dirty; the lexer cleans it up into a nice and simple Token-ized representation.
   Stage 2: Inflator
    Wherein embarks our hero (the language :P) to contrive distinction across the vast gulfs of tokens; producing meaning from chaos,
    storing Vec<Token>s in abstract structures to represent the shape of an actual sitix expression (recursively, of course!)

    sitix is a two-level language; you can think of it as essentially the inverse of the relationship between c and c preprocessor.
    this step grabs the preprocessing blocks out of a token stream and munges them together, storing the unparsed syntax (the actual
    DSL) inside the blocks. this is what allows for magic like [else] and enclosed-expressions.
   Stage 3: Parser
    A novice sees but rubble on uneven ground; a master sees a recursive grammar assembling trees to challenge any tower!

    This recursively transforms the inflated layer into a proper syntax tree. At this point all Tokens have been consumed and
    replaced with Statements, Expressions, and other such nastiness.
   Stage 4: Lexical Binding
    A novice, on a hike through the forest, passed through a clearing. Several minutes later, after much twisting and turning, he was
    surprised to walk through a clearing exactly like it. He brought this news to a master, thinking himself the discoverer of some
    great mystery- "who is to say, Wise Master, whether I walked through the same clearing, or a different one? Is it not silly
    to presume that the distinction is meaningful?"
    The master did not reply, but conjured a great wind, which brought many sheets of numbered paper to fly through the forest and alight
    in the enigmatic clearing. With this, the novice was enlightened.

    There are a bunch of edge cases where the obvious lexical meaning of a variable is quite different from what the
    interpreter actually sees. The binding step discards identifiers and ensures that every variable creation and
    access are uniquely indexable by the interpreter - simply by assigning each variable a unique number.

    As for the server - it's just a hacked-up Rouille.
*/

use std::sync::{ Arc, Mutex };
use crate::filesystem::SitixProject;


#[derive(Debug, Subcommand)]
enum Command {
    Static {
        path : PathBuf, // input directory

        /// Sets the output directory
        #[arg(short, long, value_name = "FILE")]
        output : Option<String> // the DIRECTORY to throw templated files in. templated files will have the same name as their original files,
                                // so be smart about this.
                                // sitix will never overwrite a directory that does not contain a .sitix file; this is to ensure you don't accidentally
                                // do sitix static -o . and overwite your entire project.
    },
    Dev {
        path : PathBuf, // input directory
    }
}


#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command : Command
}


fn handler(request : &rouille::Request, project : &Arc<Mutex<SitixProject>>) -> rouille::Response {
    let project = project.lock().unwrap();
    let mut interpreter = InterpreterState::new_with_standard_ffi();
    let node = if let Some(node) = project.search(None, request.url()) { node }
                else if let Some(node) = project.search(None, request.url() + "index.html") {node}
                else { return rouille::Response::empty_404(); };
    match project.into_data(node, &mut interpreter) {
        Ok(data) => {
            let data = data.to_string();
            let path = PathBuf::from(request.url());
            return rouille::Response::from_data(if let Some(ext) = path.extension() {
                match ext.to_str() {
                    Some("html") => "text/html",
                    Some("css") => "text/css",
                    Some("js") => "application/javascript",
                    _ => "text/plain"
                }
            } else { "text/html" }, data);
        },
        Err(e) => {
            return rouille::Response::text(e.to_string());
        }
    }
    rouille::Response::empty_404()
}


fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Static { path, output } => {
            let out = std::path::absolute(if let Some(output) = output { output } else { "output".to_string() }).unwrap();
            let metadata = std::fs::metadata(&path).unwrap();
            if metadata.file_type().is_dir() {
                let mut ffi = ForeignFunctionInterface::new();
                ffi.add_standard_api();
                let ffi = Arc::new(ffi);

                let mut resolver = ResolverState::new(ffi.clone());
                let mut project = filesystem::SitixProject::new(path);
                project.load_dir(None, &mut resolver).unwrap();

                let mut interpreter = InterpreterState::new(ffi.clone());

                project.render(out.into(), &mut interpreter);
            }
            else if metadata.file_type().is_file() {
                panic!("at the moment, parsing a single file is not supported.");
            }
            else {
                panic!("no such file!");
            }
        },
        Command::Dev { path } => {
            let metadata = std::fs::metadata(&path).unwrap();
            if metadata.file_type().is_dir() {
                let mut ffi = ForeignFunctionInterface::new();
                ffi.add_standard_api();
                let ffi = Arc::new(ffi);

                let mut resolver = ResolverState::new(ffi.clone());
                let mut project = filesystem::SitixProject::new(path.into());
                if let Err(e) = project.load_dir(None, &mut resolver) {
                    println!("{}", e);
                }

                let project = Arc::new(Mutex::new(project));

                std::thread::spawn({
                    let project_clone = project.clone();
                    let mut notify = project_clone.lock().unwrap().setup_inotifier();
                    move || {
                        loop {
                            let mut buffer = [0; 4096];
                            let events = notify.read_events_blocking(&mut buffer).expect("failed reading inotify events");
                            for event in events {
                                let mut project = project_clone.lock().unwrap();
                                let node = project.search_watch_descriptor(&event.wd);
                                let path = if let Some(node) = node {
                                    if let Some(path) = project.get_src_path(node) { path } else { project.get_source_dir() }
                                }
                                else {
                                    project.get_source_dir()
                                };
                                if event.mask.contains(EventMask::DELETE_SELF) {
                                    if let Some(node) = node {
                                        project.delete(node);
                                    }
                                }
                                if event.mask.contains(EventMask::CREATE) {
                                    if let Err(e) = project.track_file(node, event.name.unwrap().to_str().unwrap(), &mut resolver) {
                                        println!("{}", e);
                                    }
                                }
                                if event.mask.contains(EventMask::MOVED_TO) {
                                    if let Err(e) = project.track_file(node, event.name.unwrap().to_str().unwrap(), &mut resolver) {
                                        println!("{}", e);
                                    }
                                }
                                if event.mask.contains(EventMask::MOVED_FROM) {
                                    if let Some(file_id) = project.child_get(node, event.name.unwrap().to_str().unwrap()) {
                                        project.delete(file_id);
                                    }
                                }
                            }
                        }
                    }
                });

                println!("Starting development webserver at http://localhost:8080/");
                rouille::start_server("0.0.0.0:8080", move |request| {
                    handler(request, &project)
                });
            }
            else if metadata.file_type().is_file() {
                panic!("at the moment, running a single file development server is not supported.");
            }
            else {
                panic!("no such file!");
            }
        }
    }
}

// 1061, 1.5, 762
