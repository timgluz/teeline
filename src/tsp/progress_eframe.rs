use std::collections::{HashMap, HashSet};
use std::sync::mpsc;

use eframe;
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Stroke};

use super::kdtree::KDPoint;
use super::progress::ProgressMessage;
use super::route::Route;

type RectCoords = [f64; 4];
type Point2D = (f64, f64);
type EdgeData = (Pos2, Pos2);

const WHITE: Color32 = Color32::from_rgb(255, 255, 255);
const CITY_COLOR: Color32 = Color32::from_rgb(30, 30, 30);
const CURRENT_CITY_COLOR: Color32 = Color32::from_rgb(220, 30, 30);
const BEST_EDGE_COLOR: Color32 = Color32::from_rgb(34, 139, 34);
const SHARED_EDGE_COLOR: Color32 = Color32::from_rgb(0, 100, 0);
const UNIQUE_OPT_COLOR: Color32 = Color32::from_rgb(180, 180, 180);
const CURRENT_EDGE_COLOR: Color32 = Color32::from_rgb(30, 100, 220);
const ACTIVE_EDGE_COLOR: Color32 = Color32::from_rgb(255, 140, 0);
const STATUS_COLOR: Color32 = Color32::from_rgb(220, 30, 30);
const FONT_SIZE: f32 = 16.0;

struct NodeData {
    city_id: usize,
    pos: Pos2,
}

pub struct ProgressPlot {
    rx: mpsc::Receiver<ProgressMessage>,
    city_table: HashMap<usize, KDPoint>,
    nodes: Vec<NodeData>,
    best_edges: Vec<EdgeData>,
    best_edge_keys: HashSet<(usize, usize)>,
    optimal_edge_data: Vec<((usize, usize), EdgeData)>,
    current_edges: Vec<EdgeData>,
    current_city_id: Option<usize>,
    prev_city_id: Option<usize>,
    best_distance: f32,
    is_solved: bool,
    viewport_dimensions: ViewportDimensions,
    cities_bounding_box: RectCoords,
    status: String,
}

impl ProgressPlot {
    pub fn new_with_channel(
        cities: &[KDPoint],
        width: f64,
        height: f64,
        margin: f64,
    ) -> (Self, mpsc::Sender<ProgressMessage>) {
        let (tx, rx) = mpsc::channel();
        let mut plot = ProgressPlot {
            rx,
            city_table: HashMap::new(),
            nodes: Vec::new(),
            best_edges: Vec::new(),
            best_edge_keys: HashSet::new(),
            optimal_edge_data: Vec::new(),
            current_edges: Vec::new(),
            current_city_id: None,
            prev_city_id: None,
            best_distance: f32::MAX,
            is_solved: false,
            viewport_dimensions: ViewportDimensions::new(width, height, margin),
            cities_bounding_box: cities_bounding_box(cities),
            status: "Preparing...".to_string(),
        };
        plot.add_cities(cities);
        plot.add_nodes();
        (plot, tx)
    }

    pub fn run(self) {
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([
                    self.viewport_dimensions.width as f32,
                    self.viewport_dimensions.height as f32,
                ])
                .with_resizable(false),
            ..Default::default()
        };

        eframe::run_native(
            "Teeline - TSP solver",
            options,
            Box::new(|_cc| Ok(Box::new(self) as Box<dyn eframe::App>)),
        )
        .expect("ProgressPlot: Failed to create window");
    }

    fn handle_message(&mut self, msg: &ProgressMessage) {
        match msg {
            ProgressMessage::Done => {
                self.is_solved = true;
                self.current_city_id = None;
                self.prev_city_id = None;
                if self.best_edges.is_empty() {
                    self.best_edges = self.current_edges.clone();
                }
                self.status = if self.best_distance < f32::MAX {
                    format!("Done | best: {:.2}", self.best_distance)
                } else {
                    "Done".to_string()
                };
            }
            ProgressMessage::PathUpdate(route, distance) => {
                self.current_edges = self.build_route_edges(route);

                if *distance > 0.0 && *distance < self.best_distance {
                    self.best_distance = *distance;
                    self.best_edges = self.current_edges.clone();
                    self.best_edge_keys = Self::route_edge_keys(route);
                }

                self.status = format!("Solving... | best: {:.2}", self.best_distance);
            }
            ProgressMessage::CityChange(id) => {
                self.prev_city_id = self.current_city_id;
                self.current_city_id = Some(*id);
                self.status = "Solving...".to_string();
            }
            ProgressMessage::OptimalTour(city_ids) => {
                let route = Route::new(city_ids.as_slice());
                self.optimal_edge_data = self.build_route_edge_data(&route);
            }
            _ => {}
        }
    }

    fn add_cities(&mut self, cities: &[KDPoint]) {
        for city in cities.iter() {
            self.city_table.insert(city.id, city.clone());
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    fn add_nodes(&mut self) {
        for city in self.city_table.values() {
            let (sx, sy) = scaled_point(city, &self.cities_bounding_box, &self.viewport_dimensions);
            self.nodes.push(NodeData {
                city_id: city.id,
                pos: Pos2::new(sx as f32, sy as f32),
            });
        }
    }

    fn route_edge_keys(route: &Route) -> HashSet<(usize, usize)> {
        let path = route.route();
        let n = path.len();
        (0..n)
            .map(|i| {
                let a = path[i];
                let b = path[(i + 1) % n];
                (a.min(b), a.max(b))
            })
            .collect()
    }

    fn build_route_edge_data(&self, route: &Route) -> Vec<((usize, usize), EdgeData)> {
        let path = route.route().to_vec();
        let n = path.len();
        let mut data = Vec::new();
        for i in 0..n {
            let a = path[i];
            let b = path[(i + 1) % n];
            if let (Some(from), Some(to)) = (
                self.city_table.get(&a).cloned(),
                self.city_table.get(&b).cloned(),
            ) {
                let edge = build_edge(
                    &from,
                    &to,
                    &self.cities_bounding_box,
                    &self.viewport_dimensions,
                );
                data.push(((a.min(b), a.max(b)), edge));
            }
        }
        data
    }

    fn build_route_edges(&self, route: &Route) -> Vec<EdgeData> {
        let path = route.route().to_vec();
        let mut edges = Vec::new();
        let mut from_id = path[0];

        for to_id in path.iter().skip(1) {
            if let (Some(from), Some(to)) = (
                self.city_table.get(&from_id).cloned(),
                self.city_table.get(to_id).cloned(),
            ) {
                edges.push(build_edge(
                    &from,
                    &to,
                    &self.cities_bounding_box,
                    &self.viewport_dimensions,
                ));
                from_id = *to_id;
            }
        }

        if let (Some(from), Some(to)) = (
            self.city_table.get(&from_id).cloned(),
            self.city_table.get(&path[0]).cloned(),
        ) {
            edges.push(build_edge(
                &from,
                &to,
                &self.cities_bounding_box,
                &self.viewport_dimensions,
            ));
        }

        edges
    }
}

impl eframe::App for ProgressPlot {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(msg) = self.rx.try_recv() {
            self.handle_message(&msg);
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(WHITE))
            .show(ctx, |ui| {
                let painter = ui.painter();

                if !self.optimal_edge_data.is_empty() {
                    for ((a, b), (from, to)) in &self.optimal_edge_data {
                        let key = (*a, *b);
                        if !self.best_edge_keys.is_empty() && self.best_edge_keys.contains(&key) {
                            painter.line_segment([*from, *to], Stroke::new(6.0, SHARED_EDGE_COLOR));
                        } else {
                            painter.line_segment([*from, *to], Stroke::new(1.5, UNIQUE_OPT_COLOR));
                        }
                    }
                }

                if self.is_solved {
                    for (from, to) in &self.best_edges {
                        painter.line_segment([*from, *to], Stroke::new(4.0, BEST_EDGE_COLOR));
                    }
                } else {
                    for (from, to) in &self.current_edges {
                        painter.line_segment([*from, *to], Stroke::new(1.5, CURRENT_EDGE_COLOR));
                    }
                }

                if let (Some(prev_id), Some(curr_id)) = (self.prev_city_id, self.current_city_id) {
                    let prev_pos = self
                        .nodes
                        .iter()
                        .find(|n| n.city_id == prev_id)
                        .map(|n| n.pos);
                    let curr_pos = self
                        .nodes
                        .iter()
                        .find(|n| n.city_id == curr_id)
                        .map(|n| n.pos);
                    if let (Some(p), Some(c)) = (prev_pos, curr_pos) {
                        painter.line_segment([p, c], Stroke::new(3.0, ACTIVE_EDGE_COLOR));
                    }
                }

                for node in &self.nodes {
                    let color = if self.current_city_id == Some(node.city_id) {
                        CURRENT_CITY_COLOR
                    } else {
                        CITY_COLOR
                    };
                    painter.rect_filled(
                        Rect::from_center_size(node.pos, egui::Vec2::splat(10.0)),
                        0.0,
                        color,
                    );
                    painter.text(
                        Pos2::new(node.pos.x, node.pos.y + FONT_SIZE),
                        Align2::LEFT_TOP,
                        format!("id.{}", node.city_id),
                        FontId::proportional(FONT_SIZE),
                        color,
                    );
                }

                let margin = self.viewport_dimensions.margin as f32;
                painter.text(
                    Pos2::new(margin, margin / 2.0),
                    Align2::LEFT_TOP,
                    &self.status,
                    FontId::proportional(FONT_SIZE),
                    STATUS_COLOR,
                );

                draw_legend(
                    painter,
                    &self.viewport_dimensions,
                    !self.optimal_edge_data.is_empty(),
                );
            });

        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }
}

#[derive(Debug, Clone)]
struct ViewportDimensions {
    height: f64,
    width: f64,
    margin: f64,
}

impl ViewportDimensions {
    fn new(width: f64, height: f64, margin: f64) -> Self {
        ViewportDimensions {
            height,
            width,
            margin,
        }
    }
}

#[allow(clippy::cast_possible_truncation)]
fn build_edge(
    from: &KDPoint,
    to: &KDPoint,
    bbox: &RectCoords,
    vp: &ViewportDimensions,
) -> EdgeData {
    let (fx, fy) = scaled_point(from, bbox, vp);
    let (tx, ty) = scaled_point(to, bbox, vp);
    (
        Pos2::new(fx as f32, fy as f32),
        Pos2::new(tx as f32, ty as f32),
    )
}

fn scaled_point(point: &KDPoint, window: &RectCoords, viewport: &ViewportDimensions) -> Point2D {
    point_to_viewport(point.x() as f64, point.y() as f64, window, viewport)
}

fn cities_bounding_box(cities: &[KDPoint]) -> RectCoords {
    let mut x_min = f32::MAX;
    let mut x_max = f32::MIN;
    let mut y_min = f32::MAX;
    let mut y_max = f32::MIN;

    for city in cities.iter() {
        if let Some(x) = city.get(0) {
            if x < x_min {
                x_min = x;
            }
            if x > x_max {
                x_max = x;
            }
        }
        if let Some(y) = city.get(1) {
            if y < y_min {
                y_min = y;
            }
            if y > y_max {
                y_max = y;
            }
        }
    }

    [x_min as f64, y_min as f64, x_max as f64, y_max as f64]
}

#[allow(clippy::cast_possible_truncation)]
fn draw_legend(painter: &egui::Painter, vp: &ViewportDimensions, show_optimal: bool) {
    let x0 = vp.margin as f32;
    let line_len = 24.0_f32;
    let row_h = 20.0_f32;
    let label_x = x0 + line_len + 6.0;

    let mut entries: Vec<(Color32, &str)> = Vec::new();
    if show_optimal {
        entries.push((SHARED_EDGE_COLOR, "Shared (optimal + solver)"));
        entries.push((UNIQUE_OPT_COLOR, "Optimal only (missed)"));
    }
    entries.push((BEST_EDGE_COLOR, "Solver best"));
    entries.push((CURRENT_EDGE_COLOR, "Solver current"));
    entries.push((ACTIVE_EDGE_COLOR, "Active edge"));

    #[allow(clippy::cast_precision_loss)]
    let y0 = vp.height as f32 - vp.margin as f32 - entries.len() as f32 * row_h - 4.0;

    for (i, (color, label)) in entries.iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let y = y0 + i as f32 * row_h;
        let p0 = Pos2::new(x0, y);
        let p1 = Pos2::new(x0 + line_len, y);
        painter.line_segment([p0, p1], Stroke::new(2.5, *color));
        painter.text(
            Pos2::new(label_x, y - FONT_SIZE / 2.0),
            Align2::LEFT_TOP,
            *label,
            FontId::proportional(FONT_SIZE - 2.0),
            *color,
        );
    }
}

// converts EUC2D space into GUI coords [0..self.height, 0..self.width]
fn point_to_viewport(
    x: f64,
    y: f64,
    window: &RectCoords,
    viewport: &ViewportDimensions,
) -> Point2D {
    let x_min = window[0];
    let y_min = window[1];
    let x_max = window[2];
    let y_max = window[3];

    // TODO: research how to scale viewport itself, so we can get rid of margin here
    let x_v = (viewport.width - viewport.margin * 2.0) * (x - x_min) / (x_max - x_min);
    let y_v = (viewport.height - viewport.margin * 2.0) * (y - y_min) / (y_max - y_min);

    (x_v + viewport.margin, y_v + viewport.margin)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::kdtree::build_points;
    use crate::tsp::route::Route;

    #[test]
    fn test_route_edge_keys_triangle() {
        let route = Route::new(&[1, 2, 3]);
        let keys = ProgressPlot::route_edge_keys(&route);
        assert_eq!(keys.len(), 3);
        assert!(keys.contains(&(1, 2)));
        assert!(keys.contains(&(2, 3)));
        assert!(keys.contains(&(1, 3)));
    }

    #[test]
    fn test_route_edge_keys_normalizes_direction() {
        let route = Route::new(&[5, 2]);
        let keys = ProgressPlot::route_edge_keys(&route);
        assert!(keys.contains(&(2, 5)));
        assert!(!keys.contains(&(5, 2)));
    }

    #[test]
    fn test_cities_bounding_box_basic() {
        let cities = build_points(&[vec![0.0, 5.0], vec![10.0, 0.0], vec![3.0, 8.0]]);
        let bbox = cities_bounding_box(&cities);
        assert!((bbox[0] - 0.0).abs() < 0.01);
        assert!((bbox[1] - 0.0).abs() < 0.01);
        assert!((bbox[2] - 10.0).abs() < 0.01);
        assert!((bbox[3] - 8.0).abs() < 0.01);
    }

    #[test]
    fn test_cities_bounding_box_single_city() {
        let cities = build_points(&[vec![7.0, 3.0]]);
        let bbox = cities_bounding_box(&cities);
        assert!((bbox[0] - 7.0).abs() < 0.01);
        assert!((bbox[2] - 7.0).abs() < 0.01);
    }

    #[test]
    fn test_point_to_viewport_corner() {
        let window = [0.0, 0.0, 10.0, 10.0];
        let vp = ViewportDimensions::new(100.0, 100.0, 10.0);
        let (vx, vy) = point_to_viewport(0.0, 0.0, &window, &vp);
        assert!((vx - 10.0).abs() < 0.01);
        assert!((vy - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_point_to_viewport_far_corner() {
        let window = [0.0, 0.0, 10.0, 10.0];
        let vp = ViewportDimensions::new(100.0, 100.0, 10.0);
        let (vx, vy) = point_to_viewport(10.0, 10.0, &window, &vp);
        assert!((vx - 90.0).abs() < 0.01);
        assert!((vy - 90.0).abs() < 0.01);
    }

    #[test]
    fn test_point_to_viewport_centre() {
        let window = [0.0, 0.0, 10.0, 10.0];
        let vp = ViewportDimensions::new(100.0, 100.0, 0.0);
        let (vx, vy) = point_to_viewport(5.0, 5.0, &window, &vp);
        assert!((vx - 50.0).abs() < 0.01);
        assert!((vy - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_new_with_channel_creates_working_channel() {
        let cities = build_points(&[vec![0.0, 0.0], vec![10.0, 5.0]]);
        let (_plot, tx) = ProgressPlot::new_with_channel(&cities, 100.0, 100.0, 10.0);
        assert!(tx.send(ProgressMessage::EpochUpdate(1)).is_ok());
    }
}
