use super::rate_graph::RateGraph;
use crate::envoy::constants::NUMBER_OF_MODULES;
use crate::envoy::surveyor_envoy::SurveyorResponse;
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

/// Structure used to manage RateGraphs for the UI. Acts in observer-like role, reading a list of messages
/// from the StatusManager and trasmitting relevant data to the graph of interest.
#[derive(Debug)]
pub struct GraphManager {
    graphs: Vec<RateGraph>,
    max_points: usize,
    time_points: VecDeque<f64>,
    update_interval: Duration,
    last_update_time: Instant,
    start_time: Instant,
}

impl GraphManager {
    /// Create a new manager
    pub fn new(max_points: usize, time_step_seconds: u64) -> Self {
        let mut graphs: Vec<RateGraph> = vec![];
        for i in 0..(NUMBER_OF_MODULES - 1) {
            graphs.push(RateGraph::new(&format!("envoy_{i}"), &max_points));
        }
        let right_now = Instant::now();
        Self {
            graphs,
            max_points,
            time_points: VecDeque::new(),
            update_interval: Duration::from_secs(time_step_seconds),
            last_update_time: right_now,
            start_time: right_now,
        }
    }

    pub fn should_update(&self) -> bool {
        (Instant::now() - self.last_update_time) >= self.update_interval
    }

    /// Read messages from the embassy, looking for SurveyorResponses. If one is found, send
    /// the rate value to the appropriate graph
    pub fn update(&mut self, statuses: &[SurveyorResponse]) {
        self.last_update_time = Instant::now();
        let ellapsed_time = self.last_update_time - self.start_time;
        if self.time_points.len() == self.max_points {
            self.time_points.pop_front();
        }
        self.time_points.push_back(ellapsed_time.as_secs_f64());
        for (id, status) in statuses.into_iter().enumerate() {
            if let Some(graph) = self.graphs.get_mut(id) {
                graph.add_point(status.data_rate);
            }
        }
    }

    /// Get all of the graphs as egui_plot::Lines
    pub fn get_line_graphs(&self) -> Vec<egui_plot::Line> {
        self.graphs
            .iter()
            .map(|g| g.get_points_to_draw(&self.time_points))
            .collect()
    }

    /// Reset all of the graphs, dumping their points
    pub fn reset(&mut self) {
        self.start_time = Instant::now();
        self.last_update_time = self.start_time;
        self.time_points.clear();
        for graph in self.graphs.iter_mut() {
            graph.reset();
        }
    }

    /// Change the maximum number of points per graph. This also resets the graphs.
    pub fn set_max_points(&mut self, max_points: &usize) {
        self.max_points = *max_points;
        for graph in self.graphs.iter_mut() {
            graph.change_max_points(max_points);
        }
    }

    pub fn get_max_points(&self) -> &usize {
        &self.max_points
    }
}
