
use petgraph::{
    data::{Build, DataMapMut},
    graph::{
        Graph,
        NodeIndex
    },
    visit::IntoNeighbors
};

mod parser;
mod primitives;


#[derive (Debug)]
struct Node {
    idx: u32,
    vis: bool
}

impl Node {
    fn new(idx: u32) -> Node {
        Node {idx: idx, vis: false }
    }
}

fn main() {
    let mut g = Graph::<Node, u32>::new();
    let n1 = g.add_node(Node::new(1));
    let n2 = g.add_node(Node::new(2));
    let n3 = g.add_node(Node::new(3));
    let n4 = g.add_node(Node::new(4));

    g.add_edge(n1, n2, 1);
    g.add_edge(n1, n3, 1);
    g.add_edge(n3, n4, 1);
    g.add_edge(n2, n4, 1);


    let mut q: Vec<NodeIndex> = vec![];
    q.push(n1);

    while !q.is_empty() {
        let u = q.remove(0);
        let node = g.node_weight_mut(u).unwrap();
        if node.vis {
            continue;
        }
        node.vis = true;
        for v in g.neighbors(u).into_iter() {
            let node = g.node_weight(v).unwrap();
            if !node.vis {
                q.push(v);
            }
        }
    }
}
