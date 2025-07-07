use crate::file::File;
use petgraph::{graph::NodeIndex, visit::EdgeRef as _};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub(crate) struct Position {
    pub x: f64,
    pub y: f64,
}

impl Position {
    fn new(x: f64, y: f64) -> Self {
        Position { x, y }
    }

    fn distance(&self, other: &Position) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}

impl super::workspace_layout::WorkspaceLayout {
    pub fn calculate_positions(&self) -> Vec<(&File, Position, Vec<Position>)> {
        if self.workspace_graph.graph.node_count() == 0 {
            return vec![];
        }

        let mut layout =
            ForceDirectedLayout::new(&self.workspace_graph.graph, 800.0, 600.0);
        layout.run(&self.workspace_graph.graph, 100);

        let mut result = Vec::new();
        for node_idx in self.workspace_graph.graph.node_indices() {
            let file = &self.workspace_graph.graph[node_idx];
            let position = layout.positions[&node_idx].clone();

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
    width: f64,
    height: f64,
    k: f64,
    temperature: f64,
    cooling_factor: f64,
}

impl ForceDirectedLayout {
    fn new(
        graph: &petgraph::Graph<File, f64, petgraph::Undirected>,
        width: f64,
        height: f64,
    ) -> Self {
        let mut positions = HashMap::new();

        let center_x = width / 2.0;
        let center_y = height / 2.0;
        let radius = width.min(height) / 2.5;

        let total = graph.node_count();
        for (i, node_idx) in graph.node_indices().enumerate() {
            let angle = (i as f64 / total as f64) * std::f64::consts::TAU;
            let x = center_x + radius * angle.cos();
            let y = center_y + radius * angle.sin();
            positions.insert(node_idx, Position::new(x, y));
        }

        let area = width * height;
        let k = (area / total as f64).sqrt();

        ForceDirectedLayout {
            positions,
            width,
            height,
            k,
            temperature: width / 10.0,
            cooling_factor: 0.95,
        }
    }

    fn calculate_repulsive_force(&self, distance: f64) -> f64 {
        if distance == 0.0 {
            return 1000.0;
        }
        (self.k * self.k) / distance
    }

    fn calculate_attractive_force(&self, distance: f64) -> f64 {
        (distance * distance) / self.k
    }

    fn iterate(&mut self, graph: &petgraph::Graph<File, f64, petgraph::Undirected>) {
        let mut displacements: HashMap<NodeIndex, (f64, f64)> =
            graph.node_indices().map(|n| (n, (0.0, 0.0))).collect();

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

        // Gravité vers le centre
        let center_x = self.width / 2.0;
        let center_y = self.height / 2.0;
        let gravity_strength = self.k * 0.02;
        let circular_spring_strength = 0.01;
        let ideal_radius = self.width.min(self.height) / 2.5;

        for node_idx in graph.node_indices() {
            let pos = &self.positions[&node_idx];
            let dx = center_x - pos.x;
            let dy = center_y - pos.y;

            let disp = displacements.get_mut(&node_idx).unwrap();
            disp.0 += dx * gravity_strength;
            disp.1 += dy * gravity_strength;

            // Force pour rester sur le cercle
            let to_center_dx = pos.x - center_x;
            let to_center_dy = pos.y - center_y;
            let dist =
                (to_center_dx * to_center_dx + to_center_dy * to_center_dy).sqrt();
            if dist > 0.0 {
                let diff = dist - ideal_radius;
                disp.0 -= (to_center_dx / dist) * diff * circular_spring_strength;
                disp.1 -= (to_center_dy / dist) * diff * circular_spring_strength;
            }
        }

        for (node_idx, (dx, dy)) in displacements {
            let displacement_length = (dx * dx + dy * dy).sqrt();
            if displacement_length > 0.0 {
                let limited_displacement = displacement_length.min(self.temperature);
                let normalized_dx = dx / displacement_length;
                let normalized_dy = dy / displacement_length;

                let pos = self.positions.get_mut(&node_idx).unwrap();
                pos.x += normalized_dx * limited_displacement;
                pos.y += normalized_dy * limited_displacement;

                pos.x = pos.x.max(0.0).min(self.width);
                pos.y = pos.y.max(0.0).min(self.height);
            }
        }

        self.temperature *= self.cooling_factor;
    }

    fn run(
        &mut self,
        graph: &petgraph::Graph<File, f64, petgraph::Undirected>,
        iterations: usize,
    ) {
        for _ in 0..iterations {
            self.iterate(graph);
        }
    }
}
