extern crate petgraph;

use std::collections::HashMap;

use db::DB;
use db::table;
use std::fmt;
use std::process;

pub enum NodeType {
    Required,
    Optional
}

impl fmt::Debug for NodeType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ref Required => write!(f, "Required Node"),
            ref Optional => write!(f, "Optional Node"),
        }
    }
}

#[derive(Debug)]
pub struct Graph<'a> {
    _graph: petgraph::Graph<NodeType, String>,
    _name_map: HashMap<String, petgraph::graph::NodeIndex<petgraph::graph::DefaultIx>>,
    _index_map: HashMap<petgraph::graph::NodeIndex<petgraph::graph::DefaultIx>, String>,
    _db:  & 'a DB
}

impl<'a> Graph<'a> {
    pub fn new(db: & 'a DB) ->  Graph {
        Graph{ _graph: petgraph::Graph::<NodeType, String>::new(),
               _name_map: HashMap::new(),
               _index_map: HashMap::new(),
               _db: db}
    }

    pub fn add_or_update_product(&mut self, name: String, node_type: NodeType) {
        match self.has_product(&name) {
            true => {
                let name_index = self._name_map[&name];
                if let (&NodeType::Optional, NodeType::Required) =
                    (self._graph.node_weight(name_index).unwrap(),  node_type) {
                       self._graph[name_index] = NodeType::Required;
                }
            },
            false => {
                let index = self._graph.add_node(node_type);
                self._name_map.insert(name.clone(), index);
                self._index_map.insert(index, name);
            }
        }
    }

    pub fn has_product(& self, name: &String) -> bool {
        self._name_map.contains_key(name)
    }

    pub fn product_versions(& self, name: &String) -> Vec<&String> {
        let mut products = Vec::new();
        let index = self._name_map[name];
        let direction = petgraph::Direction::Incoming;
        for edge in self._graph.edges_directed(index, direction) {
            products.push(edge.weight());
        }
        products
    }

    pub fn connect_products(& mut self, source: &String, target: &String, version: String) -> Result<(), &str> {
        if !self.has_product(source) {
            return Err("The specified source is not in the graph");
        }
        if !self.has_product(target) {
            return Err("The specified target is not in the graph");
        }
        let source_index = self._name_map[source];
        let target_index = self._name_map[target];
        self._graph.add_edge(source_index, target_index, version);
        Ok(())
    }

    /*pub fn add_product_by_tag(& mut self,  product: String, tag: String) {
        let table = self._db.get_table_from_tag(
    }
    
    pub fn add_product_by_version(& mut self, product: String, version: String){
    }
    */

    pub fn add_table(& mut self,  table: & table::Table,
                     version_type: table::VersionType,
                     node_type: NodeType,
                     tag: Option<& Vec<& String>>,
                     recurse: bool){
        let top = & table.name;
        self.add_or_update_product(top.clone(), node_type);
        let dependencies = match version_type {
            table::VersionType::Exact => table.exact.as_ref().unwrap_or_else(||{
                let fmt_msg = format!("Error, attempted to look up exact
                        dependency matches in table file for {}, but no
                        exact matches found", top).replace("\n", "");
                println!("{}", fmt_msg);
                process::exit(1)}),
            table::VersionType::Inexact => table.inexact.as_ref().unwrap_or_else(||{
                let fmt_msg = format!("Error, attempted to look up inexact
                        dependency matches  in table file for {}, but no
                        inexact matches found", top).replace("\n", "");
                println!("{}", fmt_msg);
               process::exit(1)})
        };
        for (k, v) in dependencies.required.iter() {
            self.add_or_update_product(k.clone(), NodeType::Required);
            if let Err(_) = self.connect_products(top, &k, v.clone()) {
                println!("There was an issue connecting products in the graph");
            }
            match (tag, recurse) {
                (Some(tagVec), true) => {
                    match version_type {
                        table::VersionType::Exact => self.add_product_by_version(k.clone(), v.clone()),
                        table::VersionType::Inexact => self.add_product_by_
                    }
                },
            }
        }
    }
}
