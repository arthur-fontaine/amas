use crate::file::File;
use petgraph::{graph::NodeIndex, visit::EdgeRef as _};
use std::collections::HashMap;
use rand::{Rng, SeedableRng as _};

#[derive(Debug, Clone)]
pub(crate) struct Position {
    pub x: f32,
    pub y: f32,
}

impl Position {
    fn new(x: f32, y: f32) -> Self {
        Position { x, y }
    }
    
    fn distance(&self, other: &Position) -> f32 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}

impl super::workspace_layout::WorkspaceLayout {
    pub fn calculate_positions(&self) -> Vec<(&File, Position, Vec<Position>)> {
        if self.workspace_graph.graph.node_count() == 0 {
            return vec![];
        }
        
        let mut layout = ForceDirectedLayout::new(&self.workspace_graph.graph, 800.0, 600.0);
        layout.run(&self.workspace_graph.graph, 100);
        
        // Convert to the expected format
        let mut result = Vec::new();
        for node_idx in self.workspace_graph.graph.node_indices() {
            let file = &self.workspace_graph.graph[node_idx];
            let position = layout.positions[&node_idx].clone();
            
            // Calculate connected node positions
            let mut connected_positions = Vec::new();
            for edge in self.workspace_graph.graph.edges(node_idx) {
                let target_idx = edge.target();
                if let Some(target_pos) = layout.positions.get(&target_idx) {
                    connected_positions.push(target_pos.clone());
                }
            }
            
            result.push((file, position, connected_positions));
        }
        
        result
    }
}

struct ForceDirectedLayout {
    positions: HashMap<NodeIndex, Position>,
    width: f32,
    height: f32,
    k: f32,
    temperature: f32,
    cooling_factor: f32,
}

impl ForceDirectedLayout {
    fn new(graph: &petgraph::Graph<File, f64, petgraph::Undirected>, width: f32, height: f32) -> Self {
        let mut positions = HashMap::new();
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        
        // Initialize random positions
        for node_idx in graph.node_indices() {
            positions.insert(
                node_idx,
                Position::new(
                    rng.random_range(0.0..width),
                    rng.random_range(0.0..height)
                )
            );
        }
        
        let node_count = graph.node_count() as f32;
        let area = width * height;
        let k = (area / node_count).sqrt();
        
        ForceDirectedLayout {
            positions,
            width,
            height,
            k,
            temperature: width / 10.0,
            cooling_factor: 0.95,
        }
    }
    
    fn calculate_repulsive_force(&self, distance: f32) -> f32 {
        if distance == 0.0 {
            return 1000.0;
        }
        (self.k * self.k) / distance
    }
    
    fn calculate_attractive_force(&self, distance: f32) -> f32 {
        (distance * distance) / self.k
    }
    
    fn iterate(&mut self, graph: &petgraph::Graph<File, f64, petgraph::Undirected>) {
        let mut displacements: HashMap<NodeIndex, (f32, f32)> = HashMap::new();
        
        // Initialize displacements
        for node_idx in graph.node_indices() {
            displacements.insert(node_idx, (0.0, 0.0));
        }
        
        // Calculate repulsive forces between all pairs of nodes
        let nodes: Vec<NodeIndex> = graph.node_indices().collect();
        for i in 0..nodes.len() {
            for j in (i + 1)..nodes.len() {
                let node_v = nodes[i];
                let node_u = nodes[j];
                
                let pos_v = self.positions[&node_v].clone();
                let pos_u = self.positions[&node_u].clone();
                
                let distance = pos_v.distance(&pos_u);
                if distance > 0.0 {
                    let repulsive_force = self.calculate_repulsive_force(distance);
                    
                    let dx = (pos_v.x - pos_u.x) / distance;
                    let dy = (pos_v.y - pos_u.y) / distance;
                    
                    let disp_v = displacements.get_mut(&node_v).unwrap();
                    disp_v.0 += dx * repulsive_force;
                    disp_v.1 += dy * repulsive_force;
                    
                    let disp_u = displacements.get_mut(&node_u).unwrap();
                    disp_u.0 -= dx * repulsive_force;
                    disp_u.1 -= dy * repulsive_force;
                }
            }
        }
        
        // Calculate attractive forces for connected nodes
        for edge in graph.edge_indices() {
            let (node_u, node_v) = graph.edge_endpoints(edge).unwrap();
            
            let pos_u = self.positions[&node_u].clone();
            let pos_v = self.positions[&node_v].clone();
            
            let distance = pos_u.distance(&pos_v);
            if distance > 0.0 {
                let attractive_force = self.calculate_attractive_force(distance);
                
                let dx = (pos_v.x - pos_u.x) / distance;
                let dy = (pos_v.y - pos_u.y) / distance;
                
                let disp_u = displacements.get_mut(&node_u).unwrap();
                disp_u.0 += dx * attractive_force;
                disp_u.1 += dy * attractive_force;
                
                let disp_v = displacements.get_mut(&node_v).unwrap();
                disp_v.0 -= dx * attractive_force;
                disp_v.1 -= dy * attractive_force;
            }
        }
        
        // Apply displacements with temperature constraint
        for (node_idx, (dx, dy)) in displacements {
            let displacement_length = (dx * dx + dy * dy).sqrt();
            if displacement_length > 0.0 {
                let limited_displacement = displacement_length.min(self.temperature);
                let normalized_dx = dx / displacement_length;
                let normalized_dy = dy / displacement_length;
                
                let pos = self.positions.get_mut(&node_idx).unwrap();
                pos.x += normalized_dx * limited_displacement;
                pos.y += normalized_dy * limited_displacement;
                
                // Keep nodes within bounds
                pos.x = pos.x.max(0.0).min(self.width);
                pos.y = pos.y.max(0.0).min(self.height);
            }
        }
        
        // Cool down
        self.temperature *= self.cooling_factor;
    }
    
    fn run(&mut self, graph: &petgraph::Graph<File, f64, petgraph::Undirected>, iterations: usize) {
        for _ in 0..iterations {
            self.iterate(graph);
        }
    }
}

// Add this to your Cargo.toml:
// [dependencies]
// petgraph = "0.6"
// rand = "0.8"