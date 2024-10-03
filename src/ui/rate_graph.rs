use egui_plot::Line;
use std::collections::VecDeque;

/// Implementation of a graph for our data. Under the hood, it's just a double
/// ended queue of data. If the queue reaches the maximum allowed size, then the oldest
/// data point is dropped to add the new one (creates the ticker-tape effect).
#[derive(Debug)]
pub struct RateGraph {
    points: VecDeque<f64>,
    max_points: usize,
    name: String,
}

impl RateGraph {
    /// Create a named graph with a max size
    ///
    /// Note: time increment is hard coded to match the surveyor, should probably fix that.
    pub fn new(name: &str, max_points: &usize) -> Self {
        Self {
            points: VecDeque::with_capacity(*max_points),
            max_points: *max_points,
            name: String::from(name),
        }
    }

    /// Add a point to the graph, removing the earliest point if the capacity is reached
    pub fn add_point(&mut self, rate: f64) {
        if self.points.len() == self.max_points {
            self.points.pop_front();
        }
        self.points.push_back(rate);
    }

    /// Convert the data to a egui_plot::Line.
    pub fn get_points_to_draw(&self, times: &VecDeque<f64>) -> Line {
        Line::new(
            times
                .iter()
                .zip(self.points.iter())
                .map(|(time, rate)| [*time, *rate])
                .collect::<Vec<[f64; 2]>>(),
        )
        .name(&self.name)
    }

    /// Reset the graph, deleting all points
    pub fn reset(&mut self) {
        self.points.clear();
    }

    /// Change the maximum number of points the graph can have
    pub fn change_max_points(&mut self, max_points: &usize) {
        self.max_points = *max_points;
        self.reset();
    }
}
