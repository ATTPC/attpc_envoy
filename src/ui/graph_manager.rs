use super::rate_graph::RateGraph;
use crate::envoy::constants::NUMBER_OF_MODULES;
use crate::envoy::error::EmbassyError;
use crate::envoy::message::{EmbassyMessage, MessageKind};
use crate::envoy::surveyor_envoy::SurveyorResponse;

/// Structure used to manage RateGraphs for the UI. Acts in observer-like role, reading a list of messages
/// from the embassy and trasmitting relevant data to the graph of interest.
#[derive(Debug)]
pub struct GraphManager {
    graphs: Vec<RateGraph>,
    max_points: usize,
}

impl GraphManager {
    /// Create a new manager
    pub fn new(max_points: usize) -> Self {
        let mut graphs: Vec<RateGraph> = vec![];
        for i in 0..(NUMBER_OF_MODULES - 1) {
            graphs.push(RateGraph::new(&format!("envoy_{i}"), &max_points));
        }
        Self { graphs, max_points }
    }

    /// Read messages from the embassy, looking for SurveyorResponses. If one is found, send
    /// the rate value to the appropriate graph
    pub fn handle_messages(&mut self, messages: &[EmbassyMessage]) -> Result<(), EmbassyError> {
        for message in messages {
            match message.kind {
                MessageKind::Surveyor => {
                    if let Some(graph) = self.graphs.get_mut(message.id) {
                        let response: SurveyorResponse = message.try_into()?;
                        graph.add_point(response.data_rate);
                    }
                }
                _ => continue,
            };
        }

        Ok(())
    }

    /// Get all of the graphs as egui_plot::Lines
    pub fn get_line_graphs(&self) -> Vec<egui_plot::Line> {
        self.graphs.iter().map(|g| g.get_points_to_draw()).collect()
    }

    /// Reset all of the graphs, dumping their points
    pub fn reset_graphs(&mut self) {
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
