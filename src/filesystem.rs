// filesystem utilities.
// this contains the loader abstractions that make sitix work!

// the sitix project is just a Vec of Nodes.
// each node has a usize id, which are used as indexes to find parents and children inside the
// SitixRootTree


use std::path::PathBuf;
use std::collections::HashMap;
use crate::ast::SitixExpression;
use std::io::{Write, Read};
use crate::resolve::ResolverState;
use crate::lexer;
use crate::parse;
use crate::inflate::*;
use crate::error::*;
use crate::interpret::{ InterpreterState, Data };
use inotify::{ Inotify, WatchMask, WatchDescriptor };
use std::sync::{ Arc, Mutex };


pub struct SitixProject {
    nodes : Vec<Node>,
    sourcedir : PathBuf,
    inotify_watches : HashMap<WatchDescriptor, usize>, // map watch descriptors to nodes.
    page_data : Arc<Mutex<HashMap<usize, HashMap<String, Data>>>>
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
    },
    Deleted // TODO: make this not awful
}


impl SitixProject {
    pub fn new(sourcedir : PathBuf) -> Self {
        Self {
            nodes : vec![],
            sourcedir,
            inotify_watches : HashMap::new(),
            page_data : Arc::new(Mutex::new(HashMap::new()))
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
            Node::DataFile { name, .. } => name.clone(),
            Node::Deleted => { return None; }
        })
    }

    pub fn get_parent(&self, id : usize) -> Option<usize> {
        match self.nodes.get(id)? {
            Node::Directory { parent, .. } => parent,
            Node::ObjectFile { parent, .. } => parent,
            Node::DataFile { parent, .. } => parent,
            Node::Deleted => { return None; }
        }.clone()
    }

    fn setchild(&mut self, child : usize, parent : usize) {
        if let Some(parent) = self.nodes.get_mut(parent) {
            if let Node::Directory {children, ..} = parent {
                children.push(child);
            }
        }
    }

    pub fn load_dir(&mut self, childof : Option<usize>, resolver : &mut ResolverState) -> Result<(), Box<dyn std::error::Error>> { // recursively load a source directory
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
                self.load_dir(Some(id), resolver)?;
            }
            else {
                self.nodes.push(self.load_file(childof, child.path(), resolver)?);
                id = self.nodes.len() - 1;
            }
            if let Some(childof) = childof {
                self.setchild(id, childof);
            }
        }
        Ok(())
    }

    fn load_file(&self, parent : Option<usize>, path : PathBuf, resolver : &mut ResolverState) -> Result<Node, Box<dyn std::error::Error>> {
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

    pub fn track_file(&mut self, parent : Option<usize>, name : &str, resolver : &mut ResolverState) -> Result<(), Box<dyn std::error::Error>> {
        let mut path = if let Some(parent) = parent { self.get_src_path(parent).unwrap() } else { self.sourcedir.clone() };
        path.push(name);
        let file = self.load_file(parent, path, resolver)?;
        self.nodes.push(file);
        Ok(())
    }

    fn parse_file(path : PathBuf, resolver : &mut ResolverState) -> Result<SitixExpression, Box<dyn std::error::Error>> {
        let file = lexer::FileReader::open(&path);
        let tokens = lexer::lexer(file)?;

        let mut token_buffer = parse::TokenReader::new(tokens);
        let mut inflated = SitixTree::root(&mut token_buffer)?;

        let ast = inflated.parse(Some(path.file_name().unwrap().to_str().unwrap().to_string()))?;

        let ast = ast.resolve(resolver);
        resolver.seal();
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
                        file.write_all(expr.interpret(i, node_index, self)?.to_string().as_bytes()).unwrap();
                    }
                },
                Node::DataFile { source_path_abs, .. } => {
                    std::fs::copy(source_path_abs, path).unwrap();
                },
                Node::Deleted => panic!("unreachable")
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

    fn find_uphill(&self, from : Option<usize>, name : &str) -> Option<usize> { // from must be the PARENT of the node we're walking up from
        if let Some(child) = self.child_get(from, name) {
            return Some(child);
        }
        else if let Some(from) = from {
            return self.find_uphill(self.get_parent(from), name);
        }
        None
    }

    pub fn child_get(&self, parent : Option<usize>, name : &str) -> Option<usize> {
        if let Some(parent) = parent {
            if let Some(Node::Directory { children, .. }) = self.nodes.get(parent) {
                for child in children {
                    if self.get_name(*child).unwrap() == name {
                        return Some(*child);
                    }
                }
            }
        }
        else {
            for i in 0..self.nodes.len() {
                if let Some(node_name) = self.get_name(i) {
                    if node_name == name {
                        return Some(i);
                    }
                }
            }
        }
        None
    }

    pub fn search(&self, from : Option<usize>, name : String) -> Option<usize> {
        let name : Vec<String> = name.split("/").map(|d| d.to_string()).collect();
        let starting_point = if name[0] == "" { // leading slash
            None
        } else {
            self.find_uphill(from, &name[0])
        };
        if name.len() > 1 {
            let mut cont = starting_point;
            for subname in &name[1..] {
                cont = self.child_get(cont, &subname);
            }
            cont
        } else {
            starting_point
        }
    }

    pub fn into_data(&self, node : usize, i : &mut InterpreterState) -> SitixResult<Data> {
        Ok(match self.nodes.get(node).unwrap() {
            Node::Directory { children, .. } => {
                let mut to_vec = vec![];
                for child in children {
                    to_vec.push(self.into_data(*child, i)?);
                }
                Data::table_from_vec(to_vec)
            },
            Node::ObjectFile { expr, .. } => {
                expr.interpret(i, node, self)?
            },
            Node::DataFile { source_path_abs, .. } => {
                Data::String(std::fs::read_to_string(source_path_abs).unwrap())
            },
            Node::Deleted => panic!("unreachable")
        })
    }

    pub fn setup_inotifier(&mut self) -> Inotify { // build an inotify watch tree by visiting every node
        let inotify = Inotify::init().unwrap();
        inotify.watches().add(&self.sourcedir, WatchMask::CREATE | WatchMask::MOVED_TO | WatchMask::MOVED_FROM).expect("failed to set up file watcher");
        for node_index in 0..self.nodes.len() {
            let watch = inotify.watches().add(self.get_src_path(node_index).unwrap(), WatchMask::ALL_EVENTS).expect("failed to set up file watcher");
            self.inotify_watches.insert(watch, node_index);
        }
        inotify
    }

    pub fn search_watch_descriptor(&self, wd : &WatchDescriptor) -> Option<usize> {
        self.inotify_watches.get(wd).copied()
    }

    pub fn delete(&mut self, node : usize) {
        self.nodes[node] = Node::Deleted;
        for ind in 0..self.nodes.len() {
            if let Some(parent) = self.get_parent(ind) {
                if let Some(Node::Directory { children, .. }) = self.nodes.get_mut(parent) {
                    children.retain(|child| {
                        *child != node
                    });
                }
            }
        }
        self.inotify_watches.retain(|_, n| *n != node);
    }

    pub fn get_page_data(&self, node : usize, key : String) -> Option<Data> {
        self.page_data.lock().unwrap().get(&node)?.get(&key).cloned()
    }

    pub fn set_page_data(&self, node : usize, key : String, value : Data) {
        let mut pgdat = self.page_data.lock().unwrap();
        if let None = pgdat.get(&node) {
            pgdat.insert(node, HashMap::new());
        }
        pgdat.get_mut(&node).unwrap().insert(key, value);
    }
}

