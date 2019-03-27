/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 * Copyright Nate Lust 2018*/

pub extern crate petgraph;
use fnv::FnvHashMap;

use std::collections::HashSet;

use crate::db::graph::petgraph::visit::Walker;
use crate::db::table;
use crate::db::DB;
use std::fmt;

/**!
 The module graph contains all the machinery for turning products described
 by table files into a relational graph of those products.
*/

/// Describes if a node in the graph represents an optional dependency, or
/// a required dependency
#[derive(Clone)]
pub enum NodeType {
    Required,
    Optional,
}

/// A string representation of a node in the graph
impl fmt::Debug for NodeType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NodeType::Required => write!(f, "Required Node"),
            NodeType::Optional => write!(f, "Optional Node"),
        }
    }
}

/// Graph is a structure that holds the relational information between products, and
/// has methods to add products to the relational graph
#[derive(Debug)]
pub struct Graph<'a> {
    _graph: petgraph::Graph<NodeType, String>,
    _name_map: FnvHashMap<String, petgraph::graph::NodeIndex<petgraph::graph::DefaultIx>>,
    _index_map: FnvHashMap<petgraph::graph::NodeIndex<petgraph::graph::DefaultIx>, String>,
    _db: &'a DB,
    _processed: HashSet<String>,
}

impl<'a> Graph<'a> {
    /// Created a new graph that will be associated with the specified database
    pub fn new(db: &'a DB) -> Graph {
        Graph {
            _graph: petgraph::Graph::<NodeType, String>::new(),
            _name_map: FnvHashMap::default(),
            _index_map: FnvHashMap::default(),
            _db: db,
            _processed: HashSet::new(),
        }
    }
    /// Resolves the index of a graph node into a string of the product name at that node
    pub fn get_name(
        &self,
        number: petgraph::graph::NodeIndex<petgraph::graph::DefaultIx>,
    ) -> String {
        return self._index_map[&number].clone();
    }

    /// Add a product to the graph, or update the node type of that product if it already exists
    pub fn add_or_update_product(&mut self, name: String, node_type: NodeType) {
        match self.has_product(&name) {
            true => {
                let name_index = self._name_map[&name];
                if let (&NodeType::Optional, NodeType::Required) =
                    (self._graph.node_weight(name_index).unwrap(), node_type)
                {
                    self._graph[name_index] = NodeType::Required;
                }
            }
            false => {
                let index = self._graph.add_node(node_type);
                self._name_map.insert(name.clone(), index);
                self._index_map.insert(index, name);
            }
        }
    }

    /// Checks if the graph contains a given product
    pub fn has_product(&self, name: &String) -> bool {
        self._name_map.contains_key(name)
    }

    /// Returns all the different versions of a product that the graph describes.
    /// I.E. different nodes in the graph may point to a given product as a dependency
    /// with different version of that dependency listed as a requirement
    pub fn product_versions(&self, name: &String) -> Vec<&String> {
        let mut products = Vec::new();
        let index = self._name_map[name];
        let direction = petgraph::Direction::Incoming;
        for edge in self._graph.edges_directed(index, direction) {
            products.push(edge.weight());
        }
        products
    }

    /// Determines if a given node is listed as an optional node in the graph
    pub fn is_optional(&self, name: &String) -> bool {
        let node = self._name_map[name];
        let weight = self._graph.node_weight(node);
        match weight {
            Some(NodeType::Optional) => true,
            _ => false,
        }
    }

    /// Connects two products (nodes) in the graph together with a specific version. Note that this
    /// is a directional graph so the version requirement will point from the source to the target
    /// node
    pub fn connect_products(
        &mut self,
        source: &String,
        target: &String,
        version: String,
    ) -> Result<(), &str> {
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

    /// Add a product to the graph specified by a given tag. This tag is looked up in the database
    /// associated with this graph and resolved into a table file. Optionally add in the dependencies from the table file if recurse is true
    pub fn add_product_by_tag(
        &mut self,
        product: String,
        tag: &Vec<&str>,
        version_type: table::VersionType,
        node_type: NodeType,
        recurse: bool,
    ) {
        if !self._processed.contains(&product) {
            let result = self._db.get_table_from_tag(&product, tag);
            if let Some(table) = result {
                self.add_table(&table, version_type, node_type, Some(tag), recurse);
            }
        }
    }

    /// Add a product to the graph specified by a given version. This version is looked up in the database
    /// associated with this graph and resolved into a table file. Optionally add in the
    /// dependencies from the table file if recurse is true
    pub fn add_product_by_version(
        &mut self,
        product: String,
        version: String,
        version_type: table::VersionType,
        node_type: NodeType,
        recurse: bool,
    ) {
        if !self._processed.contains(&product) {
            let result = self._db.get_table_from_version(&product, &version);
            if let Some(table) = result {
                self.add_table(&table, version_type, node_type, None, recurse);
            }
        }
    }

    /// Add a specific table into the graph of products. Optionally add in the
    /// dependencies from the table file if recurse is true
    pub fn add_table(
        &mut self,
        table: &table::Table,
        version_type: table::VersionType,
        node_type: NodeType,
        tag: Option<&Vec<&str>>,
        recurse: bool,
    ) {
        let top = &table.name;

        self.add_or_update_product(top.clone(), node_type);

        let dependencies = match version_type {
            table::VersionType::Exact => table.exact.as_ref(),
            table::VersionType::Inexact => table.inexact.as_ref(),
        };
        // This means that there are no dependencies of the required type, and so the function
        // should return.
        if let None = dependencies {
            crate::debug!("No dependencies found for {}", top);
            return;
        }
        // If there are dependencies for the version type, loop over the required and optional
        // dependencies
        let dep_unwrap = dependencies.unwrap();
        crate::debug!("{} has dependencies of {:?}", top, dep_unwrap);
        for (dep_vec, node_type) in vec![&dep_unwrap.required, &dep_unwrap.optional]
            .iter()
            .zip(vec![NodeType::Required, NodeType::Optional])
        {
            for (k, v) in dep_vec.iter() {
                self.add_or_update_product(k.clone(), node_type.clone());
                if let Err(_) = self.connect_products(top, &k, v.clone()) {
                    crate::warn!("There was an issue connecting products in the graph, topological walks my be incorrect");
                }

                match (&version_type, tag, recurse) {
                    (table::VersionType::Inexact, Some(tag_vec), true) => self.add_product_by_tag(
                        k.clone(),
                        tag_vec,
                        table::VersionType::Inexact,
                        node_type.clone(),
                        recurse,
                    ),
                    (table::VersionType::Exact, _, true) => self.add_product_by_version(
                        k.clone(),
                        v.clone(),
                        table::VersionType::Exact,
                        node_type.clone(),
                        recurse,
                    ),
                    _ => {}
                }
            }
        }
        self._processed.insert(top.clone());
    }

    /// Iterates though the nodes of the graph
    pub fn iter(
        &self,
    ) -> petgraph::visit::WalkerIter<
        petgraph::visit::Topo<
            <petgraph::Graph<NodeType, String> as petgraph::visit::GraphBase>::NodeId,
            <petgraph::Graph<NodeType, String> as petgraph::visit::Visitable>::Map,
        >,
        &petgraph::Graph<NodeType, String>,
    > {
        let topo = petgraph::visit::Topo::new(&self._graph);
        return topo.iter(&self._graph);
    }
}
