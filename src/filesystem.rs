// filesystem utilities.
// this contains the loader abstractions that make sitix work!

// the sitix project is just a Vec of Nodes.
// each node has a usize id, which are used as indexes to find parents and children inside the
// SitixRootTree


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


pub struct SitixProject {
    nodes : Vec<Node>,
    sourcedir : PathBuf
}


#[derive(Debug)]
pub enum Node {
    Directory {
        name : String,
        parent : Option<usize>,
        children : Vec<usize>
    },
    ObjectFile { // a file with an opening phrase [?] or [!]
        name : String,
        expr : SitixExpression,
        parent : Option<usize>,
        render : bool // if the opening phrase is [!], this will be true
    },
    DataFile { // a file with no opening phrase or the opening phrase [@]
        name : String,
        parent : Option<usize>,
        source_path_abs : PathBuf, // absolute source path
    }
}


impl SitixProject {
    pub fn new(sourcedir : PathBuf) -> Self {
        Self {
            nodes : vec![],
            sourcedir
        }
    }

    pub fn get_path(&self, id : usize, root : PathBuf) -> Option<PathBuf> { // perform recursive lookups to transform a given node id into its filename
        let mut root = if let Some(parent) = self.get_parent(id) { // find what comes *before* this name
            self.get_path(parent, root)?
        }
        else {
            root
        };
        root.push(self.get_name(id)?); // add our name to it
        Some(root) // return the combo!
    }

    pub fn get_src_path(&self, id : usize) -> Option<PathBuf> {
        self.get_path(id, self.sourcedir.clone())
    }

    pub fn get_name(&self, id : usize) -> Option<String> {
        Some(match self.nodes.get(id)? {
            Node::Directory { name, .. } => name.clone(),
            Node::ObjectFile { name, .. } => name.clone(),
            Node::DataFile { name, .. } => name.clone()
        })
    }

    pub fn get_parent(&self, id : usize) -> Option<usize> {
        match self.nodes.get(id)? {
            Node::Directory { parent, .. } => parent,
            Node::ObjectFile { parent, .. } => parent,
            Node::DataFile { parent, .. } => parent
        }.clone()
    }

    fn setchild(&mut self, child : usize, parent : usize) {
        if let Some(parent) = self.nodes.get_mut(parent) {
            if let Node::Directory {children, ..} = parent {
                children.push(child);
            }
        }
    }

    pub fn load_dir(&mut self, childof : Option<usize>, resolver : &mut ResolverState) { // recursively load a source directory
        let root = if let Some(childof) = childof { self.get_src_path(childof).unwrap() } else { self.sourcedir.clone() };
        for child in std::fs::read_dir(root).unwrap() {
            let child = child.unwrap();
            let id;
            if child.path().is_dir() {
                self.nodes.push(Node::Directory {
                    name : child.path().file_name().unwrap().to_str().unwrap().to_string(),
                    parent : childof,
                    children : vec![]
                });
                id = self.nodes.len() - 1;
                self.load_dir(Some(id), resolver);
            }
            else {
                self.nodes.push(self.load_file(childof, child.path(), resolver).unwrap());
                id = self.nodes.len() - 1;
            }
            if let Some(childof) = childof {
                self.setchild(id, childof);
            }
        }
    }

    fn load_file(&self, parent : Option<usize>, path : PathBuf, resolver : &mut ResolverState) -> std::io::Result<Node> {
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
                        return Ok(Node::ObjectFile {
                            expr : Self::parse_file(path.clone(), resolver)?,
                            name : path.file_name().unwrap().to_str().unwrap().to_string(),
                            render : opening_phrase[1] == b'!',
                            parent
                        });
                    }
                    _ => {}
                }
            }
        }
        Ok(Node::DataFile {
            source_path_abs : std::path::absolute(&path).unwrap(),
            name : path.file_name().unwrap().to_str().unwrap().to_string(),
            parent
        })
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

    fn render_node(&self, out : PathBuf, node_index : usize, i : &mut InterpreterState) -> Result<(), Box<dyn std::error::Error>> {
        let path = self.get_path(node_index, out).unwrap();
        if let Some(node) = self.nodes.get(node_index) {
            match node {
                Node::Directory { .. } => {
                    std::fs::create_dir_all(path)?;
                },
                Node::ObjectFile { expr, render, .. } => {
                    if *render {
                        let mut file = std::fs::File::create(path)?;
                        file.write_all(expr.interpret(i, node_index, self)?.to_string().as_bytes());
                    }
                },
                Node::DataFile { source_path_abs, .. } => {
                    std::fs::copy(source_path_abs, path);
                }
            }
        }
        Ok(())
    }

    pub fn render(&self, out : PathBuf, i : &mut InterpreterState) {
        for node in 0..self.nodes.len() {
            if let Err(e) = self.render_node(out.clone(), node, i) {
                println!("{}", e);
            }
        }
    }

    fn find_uphill(&self, from : Option<usize>, name : String) -> Option<usize> { // from must be the PARENT of the node we're walking up from
        if let Some(from) = from {
            if let Some(Node::Directory { children, .. }) = self.nodes.get(from) {
                for child in children {
                    if self.get_name(*child).unwrap() == name {
                        return Some(*child);
                    }
                }
                return self.find_uphill(self.get_parent(from), name);
            }
        }
        else {
            for i in 0..self.nodes.len() {
                if let None = self.get_parent(i) {
                    if self.get_name(i).unwrap() == name {
                        return Some(i);
                    }
                }
            }
        }
        None
    }

    fn child_get(&self, parent : usize, name : String) -> Option<usize> {
        if let Some(Node::Directory { children, .. }) = self.nodes.get(parent) {
            for child in children {
                if self.get_name(*child).unwrap() == name {
                    return Some(*child);
                }
            }
        }
        None
    }

    pub fn search(&self, mut from : Option<usize>, name : String) -> Option<usize> {
        let name : Vec<String> = name.split("/").map(|d| d.to_string()).collect();
        if name[0] == "" {
            from = None;
        }
        from = if let Some(from) = from { self.get_parent(from) } else { None };
        let mut point = self.find_uphill(from, name[0].clone())?;
        if name.len() > 1 {
            for child_name in &name[1..] {
                point = self.child_get(point, child_name.clone())?;
            }
        }
        Some(point)
    }

    pub fn into_data(&self, node : usize, i : &mut InterpreterState) -> Option<Data> {
        Some(match self.nodes.get(node)? {
            Node::Directory { children, .. } => {
                let mut to_vec = vec![];
                for child in children {
                    if let Some(data) = self.into_data(*child, i) {
                        to_vec.push(data);
                    }
                }
                Data::table_from_vec(to_vec)
            },
            Node::ObjectFile { expr, render, .. } => {
                expr.interpret(i, node, self).unwrap()
            },
            Node::DataFile { source_path_abs, .. } => {
                Data::String(std::fs::read_to_string(source_path_abs).unwrap())
            }
        })
    }
}

