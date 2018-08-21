pub extern crate petgraph;
use db::fnv::FnvHashMap;

use std::collections::HashSet;

use db::DB;
use db::table;
use std::fmt;
use db::graph::petgraph::visit::Walker;

#[derive(Clone)]
pub enum NodeType {
    Required,
    Optional
}

impl fmt::Debug for NodeType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NodeType::Required => write!(f, "Required Node"),
            NodeType::Optional => write!(f, "Optional Node"),
        }
    }
}

#[derive(Debug)]
pub struct Graph<'a> {
    _graph: petgraph::Graph<NodeType, String>,
    _name_map: FnvHashMap<String, petgraph::graph::NodeIndex<petgraph::graph::DefaultIx>>,
    _index_map: FnvHashMap<petgraph::graph::NodeIndex<petgraph::graph::DefaultIx>, String>,
    _db:  & 'a DB,
    _processed: HashSet<String>
}

impl<'a> Graph<'a> {
    pub fn new(db: & 'a DB) ->  Graph {
        Graph{ _graph: petgraph::Graph::<NodeType, String>::new(),
               _name_map: FnvHashMap::default(),
               _index_map: FnvHashMap::default(),
               _db: db,
               _processed: HashSet::new()}
    }
    pub fn get_name(& self, number: petgraph::graph::NodeIndex<petgraph::graph::DefaultIx>) -> String {
        return self._index_map[&number].clone()
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

    pub fn add_product_by_tag(& mut self,  product: String,
                              tag: & Vec<& String>,
                              version_type: table::VersionType,
                              node_type: NodeType,
                              recurse: bool) {
        let result = self._db.get_table_from_tag(& product, tag.clone());
        if let Some(table) = result {
            self.add_table(&table, version_type, node_type, Some(tag), recurse);
        }
    }
    
    pub fn add_product_by_version(& mut self, product: String, version: String,
                                  version_type: table::VersionType, node_type: NodeType, recurse: bool){
        let result = self._db.get_table_from_version(& product, & version);
        if let Some(table) = result {
            if !self._processed.contains(&table.name) {
                self.add_table(&table, version_type, node_type, None, recurse);
            }
        }
    }

    pub fn add_table(& mut self,  table: & table::Table,
                     version_type: table::VersionType,
                     node_type: NodeType,
                     tag: Option<& Vec<& String>>,
                     recurse: bool){
        let top = & table.name;

        self.add_or_update_product(top.clone(), node_type);

        let dependencies = match version_type {
            table::VersionType::Exact => table.exact.as_ref(),
            table::VersionType::Inexact => table.inexact.as_ref()
        };
        // This means that there are no dependencies of the required type, and so the function
        // should return.
        if let None = dependencies {
            return
        }
        // If there are dependencies for the version type, loop over the required and optional
        // dependencies
        let dep_unwrap = dependencies.unwrap();
        for (dep_vec, node_type) in vec![&dep_unwrap.required, &dep_unwrap.optional].iter().zip(
                                        vec![NodeType::Required, NodeType::Optional]) {
            for (k, v) in dep_vec.iter() {
                self.add_or_update_product(k.clone(), node_type.clone());
                if let Err(_) = self.connect_products(top, &k, v.clone()) {
                    println!("There was an issue connecting products in the graph");
                }
                
                match (&version_type, tag, recurse) {
                    (table::VersionType::Inexact, Some(tag_vec), true) => {
                        self.add_product_by_tag(k.clone(), tag_vec, table::VersionType::Inexact,
                                                node_type.clone(), recurse)
                        },
                    (table::VersionType::Exact, _, true) => {
                        self.add_product_by_version(k.clone(), v.clone(), table::VersionType::Exact,
                                                    node_type.clone(), recurse)
                        },
                    _ => {}
                }
            }
        }
        self._processed.insert(top.clone());
    }

    pub fn iter(& self) -> petgraph::visit::WalkerIter<petgraph::visit::Topo<<petgraph::Graph<NodeType, String> as petgraph::visit::GraphBase>::NodeId, <petgraph::Graph<NodeType, String> as petgraph::visit::Visitable>::Map>, &petgraph::Graph<NodeType, String>>{
        let topo = petgraph::visit::Topo::new(&self._graph);
        return topo.iter(&self._graph)
    }
}
