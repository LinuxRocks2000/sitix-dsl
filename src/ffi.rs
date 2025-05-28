// structure for handling the foreign function interface
use std::collections::HashMap;
use crate::interpret::{ Data, SitixFunction, InterpreterState };
use crate::error::*;
use crate::filesystem::SitixProject;


#[derive(Debug)]
pub struct ForeignFunctionInterface {
    name_to_index : HashMap<String, usize>,
    index_to_object : HashMap<usize, Data>,
    pub top_index : usize
}


impl ForeignFunctionInterface {
    pub fn new() -> Self {
        Self {
            name_to_index : HashMap::new(),
            index_to_object : HashMap::new(),
            top_index : 0
        }
    }

    pub fn add(&mut self, name : String, data : Data) {
        self.name_to_index.insert(name, self.top_index);
        self.index_to_object.insert(self.top_index, data);
        self.top_index += 1;
    }

    pub fn find(&self, name : &String) -> Option<usize> {
        Some(self.name_to_index.get(name)?.clone())
    }

    pub fn get(&self, index : usize) -> Option<Data> {
        Some(self.index_to_object.get(&index)?.clone())
    }

    pub fn add_several(&mut self, to_insert : &[(String, Data)]) {
        for (name, data) in to_insert {
            self.add(name.clone(), data.clone());
        }
    }

    pub fn add_several_functions(&mut self, to_insert : &[(String, &'static dyn Fn(&mut InterpreterState, usize, &SitixProject, &[Data]) -> SitixResult<Data>)]) {
        for (name, data) in to_insert {
            self.add(name.to_string(), Data::Function(SitixFunction::Builtin(*data)));
        }
    }

    pub fn add_standard_api(&mut self) {
        self.add_several_functions(&[
            ("print".to_string(), &|i, _, _, args| {
                for arg in args {
                    if let Ok(data) = i.deref(arg.clone()) {
                        print!("{} ", data.to_string());
                    }
                }
                println!("");
                Ok(Data::Nil)
            }),
            ("include".to_string(), &|i, node, project, args| {
                let old_export_table = i.export_table.clone();
                i.export_table = HashMap::new();
                let out_node = project.search(Some(node), i.deref(args[0].clone()).unwrap().to_string()).unwrap();
                let ret = project.into_data(out_node, i).unwrap();
                i.export_table = old_export_table;
                println!("including {:?}", ret);
                Ok(ret)
            })
        ]);
    }
}
