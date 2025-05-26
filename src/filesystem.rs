// filesystem utilities.
// this contains the loader abstractions that make sitix work!
// all path operations are relative and should occur from inside the appropriate directory.
// when loading, for instance, you must have CWD = the root directory
// when rendering, you must have CWD = the output directory


use std::path::{PathBuf, Path};
use std::sync::Arc;
use crate::ast::SitixExpression;
use std::io::{Write, Read};
use crate::resolve::ResolverState;
use crate::lexer;
use crate::parse;
use crate::inflate::*;
use crate::error::*;
use crate::interpret::{ InterpreterState, Data };


#[derive(Debug)]
pub enum Node {
    Directory {
        path : PathBuf,
        children : Vec<Arc<Node>>
    },
    ObjectFile { // a file with an opening phrase [?] or [!]
        path : PathBuf,
        expr : SitixExpression,
        render : bool // if the opening phrase is [!], this will be true
    },
    DataFile { // a file with no opening phrase or the opening phrase [@]
        path : PathBuf,
        source_path_abs : PathBuf,
        render : bool // if the opening phrase is [@], this is false
        // DataFiles are not actually loaded, as they can be quite large! normally they will only be copied.
        // the only reason they will ever be loaded in RAM is that an actual sitix file used include.
        // datafiles with the .json extension will be parsed as tables for convenience upon include-ing (tables are a superset of JSON)
    }
}


impl Node {
    pub fn load(path : PathBuf, resolver : &mut ResolverState) -> std::io::Result<Arc<Node>> {
        let meta = std::fs::metadata(&path)?;
        if meta.file_type().is_dir() {
            Self::load_dir(path, resolver)
        }
        else if meta.file_type().is_file() {
            Self::load_file(path, resolver)
        }
        else {
            panic!("invalid path {:?}", path);
        }
    }

    pub fn load_file(path : PathBuf, resolver : &mut ResolverState) -> std::io::Result<Arc<Node>> {
        let mut opening_phrase = [0u8; 3];
        let count;
        {
            let mut file = std::fs::File::open(&path)?;
            count = file.read(&mut opening_phrase)?;
        }
        if count == 3 {
            if opening_phrase[0] == b'[' && opening_phrase[2] == b']' {
                match opening_phrase[1] {
                    b'!' | b'?' => {
                        return Ok(Arc::new(Node::ObjectFile {
                            expr : Self::parse_file(path.clone(), resolver)?,
                            path,
                            render : opening_phrase[1] == b'!'
                        }));
                    },
                    b'@' => {
                        return Ok(Arc::new(Node::DataFile {
                            source_path_abs : std::path::absolute(&path).unwrap(),
                            path,
                            render : false
                        }));
                    }
                    _ => {}
                }
            }
        }
        Ok(Arc::new(Node::DataFile {
            source_path_abs : std::path::absolute(&path).unwrap(),
            path,
            render : true
        }))
    }

    pub fn load_dir(path : PathBuf, resolver : &mut ResolverState) -> std::io::Result<Arc<Node>> {
        let mut children = vec![];
        for path in std::fs::read_dir(&path)? {
            children.push(Self::load(path?.path(), resolver)?);
        }
        Ok(Arc::new(
            Node::Directory {
                path,
                children
            }
        ))
    }

    fn parse_file(path : PathBuf, resolver : &mut ResolverState) -> std::io::Result<SitixExpression> {
        let file = lexer::FileReader::open(path);
        let tokens = lexer::lexer(file).unwrap();

        let mut token_buffer = parse::TokenReader::new(tokens);
        let mut inflated = SitixTree::root(&mut token_buffer).unwrap();

        let ast = inflated.parse().unwrap();

        let ast = ast.resolve(resolver);
        Ok(ast)
    }

    pub fn render(&self, interpreter : &mut InterpreterState) -> SitixResult<()> {
        match self {
            Self::Directory { path, children } => {
                std::fs::create_dir_all(path).unwrap();
                for child in children {
                    child.render(interpreter)?;
                }
            },
            Self::DataFile { path, source_path_abs, render } => {
                if *render {
                    std::fs::copy(source_path_abs, path).unwrap();
                }
            },
            Self::ObjectFile { path, expr, render } => {
                if *render {
                    let mut file = std::fs::File::create(path).unwrap();
                    let output = expr.interpret(interpreter)?.to_string();
                    file.write(output.as_bytes());
                }
            }
        }
        Ok(())
    }

    pub fn search(&self, path : impl AsRef<Path>) -> Option<Arc<Self>> {
        let path = path.as_ref();
        match self {
            Self::Directory { path : _, children } => {
                for child in children {
                    if child.get_path() == path {
                        return Some(child.clone());
                    }
                    if let Some(out) = child.search(path) {
                        return Some(out);
                    }
                }
            },
            _ => {}
        }
        None
    }

    fn get_path(&self) -> PathBuf {
        match self {
            Self::Directory { path, children : _ } => {
                path.clone()
            },
            Self::DataFile { path, .. } => {
                path.clone()
            },
            Self::ObjectFile { path, .. } => {
                path.clone()
            }
        }
    }

    pub fn into_data(&self, i : &mut InterpreterState) -> SitixResult<Data> {
        match self {
            Self::Directory { path : _, children } => {
                let mut childs = vec![];
                for child in children {
                    childs.push(child.into_data(i).unwrap());
                }
                Ok(Data::table_from_vec(childs))
            },
            Self::DataFile { path : _, source_path_abs, render : _ } => {
                Ok(Data::String(std::fs::read_to_string(source_path_abs).unwrap()))
            },
            Self::ObjectFile { path : _, expr, render : _ } => {
                expr.interpret(i)
            }
        }
    }
}
