use super::app::EnvoyApp;
use eframe::egui::{Color32, RichText, TopBottomPanel};

///Render the graph panel, the bottom of the UI
pub fn render_graph_panel(app: &mut EnvoyApp, ctx: &eframe::egui::Context) {
    TopBottomPanel::bottom("Graph_Panel").show(ctx, |ui| {
        let mut max_points = app.graphs.get_max_points().clone();
        ui.separator();
        let lines = app.graphs.get_line_graphs();
        ui.label(
            RichText::new("Data Rate Graph")
                .color(Color32::LIGHT_BLUE)
                .size(18.0),
        );
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(RichText::new("Number of Points Per Graph").size(16.0));
            ui.add(eframe::egui::DragValue::new(&mut max_points).speed(1));
        });
        ui.separator();
        if *app.graphs.get_max_points() != max_points {
            app.graphs.set_max_points(&max_points)
        }
        egui_plot::Plot::new("RatePlot")
            .view_aspect(6.0)
            .height(200.0)
            .legend(egui_plot::Legend::default().position(egui_plot::Corner::LeftTop))
            .x_axis_label(RichText::new("Time Since Run Start (s)").size(16.0))
            .y_axis_label(RichText::new("Rate (MB/s)").size(16.0))
            .show(ui, |plot_ui| {
                for line in lines {
                    plot_ui.line(line);
                }
            });
        ui.separator();
    });
}
