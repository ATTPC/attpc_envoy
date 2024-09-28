use egui_plot::Line;
use std::collections::VecDeque;

/// Implementation of a graph for our data. Under the hood, it's just a double
/// ended queue of data. If the queue reaches the maximum allowed size, then the oldest
/// data point is dropped to add the new one (creates the ticker-tape effect).
#[derive(Debug)]
pub struct RateGraph {
    points: VecDeque<f64>,
    max_points: usize,
    last_time: f64,
    time_increment: f64,
    name: String,
}

impl RateGraph {
    /// Create a named graph with a max size
    ///
    /// Note: time increment is hard coded to match the surveyor, should probably fix that.
    pub fn new(name: &str, max_points: &usize) -> Self {
        Self {
            points: VecDeque::new(),
            max_points: *max_points,
            last_time: 0.0,
            time_increment: 2.0,
            name: String::from(name),
        }
    }

    /// Add a point to the graph, removing the earliest point if the capacity is reached
    pub fn add_point(&mut self, rate: f64) {
        if self.points.len() == self.max_points {
            self.points.pop_front();
        }
        let current_time = self.last_time + self.time_increment;
        self.points.push_back(rate);
        self.last_time = current_time;
    }

    /// Convert the data to a egui_plot::Line.
    pub fn get_points_to_draw(&self) -> Line {
        let total_len = self.points.len() as i32 - 1;
        Line::new(
            self.points
                .clone()
                .into_iter()
                .enumerate()
                .rev()
                .map(|(i, rate)| [((i as i32 - total_len) as f64) * self.time_increment, rate])
                .collect::<Vec<[f64; 2]>>(),
        )
        .name(&self.name)
    }

    /// Reset the graph, deleting all points
    pub fn reset(&mut self) {
        self.points.clear();
        self.last_time = 0.0;
    }

    /// Change the maximum number of points the graph can have
    pub fn change_max_points(&mut self, max_points: &usize) {
        self.max_points = *max_points;
        self.reset();
    }
}
