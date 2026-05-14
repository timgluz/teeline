use std::sync::mpsc;
use std::sync::Mutex;
use std::sync::LazyLock;

use eframe;
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Stroke};

use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;

use super::route::Route;
use super::KDPoint;

pub type PublishChannel = Sender<ProgressMessage>;
pub type ReceiverChannel = Receiver<ProgressMessage>;
pub type PublisherFn = Arc<dyn Fn(ProgressMessage)>;

type RectCoords = [f64; 4];
type Point2D = (f64, f64);
type EdgeData = (Pos2, Pos2);

const WHITE:              Color32 = Color32::from_rgb(255, 255, 255);
const CITY_COLOR:         Color32 = Color32::from_rgb(30,  30,  30);   // near-black nodes
const CURRENT_CITY_COLOR: Color32 = Color32::from_rgb(220, 30,  30);   // red — active city
const BEST_EDGE_COLOR:    Color32 = Color32::from_rgb(34,  139, 34);   // green — best route
const CURRENT_EDGE_COLOR: Color32 = Color32::from_rgb(30,  100, 220);  // blue — exploring route
const ACTIVE_EDGE_COLOR:  Color32 = Color32::from_rgb(255, 140, 0);    // orange — current step
const STATUS_COLOR:       Color32 = Color32::from_rgb(220, 30,  30);   // red — status text
const FONT_SIZE: f32 = 16.0;

static PUBLISH_CHANNEL: LazyLock<Mutex<Option<PublishChannel>>> =
    LazyLock::new(|| Mutex::new(None));
static RECEIVER_CHANNEL: LazyLock<Mutex<Option<ReceiverChannel>>> =
    LazyLock::new(|| Mutex::new(None));

fn init_channels() {
    let (out_ch, in_ch) = mpsc::channel();
    let mut mtx = PUBLISH_CHANNEL.lock().unwrap();
    *mtx = Some(out_ch);

    let mut inmtx = RECEIVER_CHANNEL.lock().unwrap();
    *inmtx = Some(in_ch);
}

pub fn get_publisher() -> Option<PublishChannel> {
    match PUBLISH_CHANNEL.lock() {
        Ok(mtx) => mtx.clone(),
        Err(_) => None,
    }
}

pub fn send_progress(msg: ProgressMessage) {
    let publish_ch = PUBLISH_CHANNEL.lock().unwrap();
    if publish_ch.is_some() {
        publish_ch
            .clone()
            .unwrap()
            .send(msg)
            .expect("Failed to publish message");
    }
}

fn try_retrieve_message() -> Option<ProgressMessage> {
    let ch = RECEIVER_CHANNEL.lock().unwrap();
    ch.as_ref().and_then(|x| x.try_recv().ok())
}

#[derive(Debug, Clone)]
pub enum ProgressMessage {
    CityChange(usize),
    PathUpdate(Route, f32),
    EpochUpdate(usize),
    Done,
    Restart,
}

struct NodeData {
    city_id: usize,
    pos: Pos2,
}

pub struct ProgressPlot {
    city_table: HashMap<usize, KDPoint>,
    nodes: Vec<NodeData>,
    best_edges: Vec<EdgeData>,    // shown in green after Done
    current_edges: Vec<EdgeData>, // shown in blue while solving
    current_city_id: Option<usize>,
    prev_city_id: Option<usize>,
    best_distance: f32,
    is_solved: bool,
    viewport_dimensions: ViewportDimensions,
    cities_bounding_box: RectCoords,
    status: String,
}

impl ProgressPlot {
    pub fn new(cities: &[KDPoint], width: f64, height: f64, margin: f64) -> Self {
        // Initialise channels here so they are ready before the solver thread starts.
        init_channels();

        let mut plot = ProgressPlot {
            city_table: HashMap::new(),
            nodes: Vec::new(),
            best_edges: Vec::new(),
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
        plot
    }

    pub fn run(self, show_progress: bool) {
        if !show_progress {
            // Drain messages so the solver's channel send never blocks, then exit.
            loop {
                match try_retrieve_message() {
                    Some(ProgressMessage::Done) => break,
                    None => std::thread::sleep(std::time::Duration::from_millis(5)),
                    _ => {}
                }
            }
            return;
        }

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
                // If no PathUpdate with distance>0 arrived, fall back to whatever
                // route was last displayed so we always show something on finish.
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
                }

                self.status = format!("Solving... | best: {:.2}", self.best_distance);
            }
            ProgressMessage::CityChange(id) => {
                self.prev_city_id = self.current_city_id;
                self.current_city_id = Some(*id);
                self.status = "Solving...".to_string();
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

    fn build_route_edges(&self, route: &Route) -> Vec<EdgeData> {
        let path = route.route().to_vec();
        let mut edges = Vec::new();
        let mut from_id = path[0];

        for to_id in path.iter().skip(1) {
            if let (Some(from), Some(to)) = (
                self.city_table.get(&from_id).cloned(),
                self.city_table.get(to_id).cloned(),
            ) {
                edges.push(build_edge(&from, &to, &self.cities_bounding_box, &self.viewport_dimensions));
                from_id = *to_id;
            }
        }

        if let (Some(from), Some(to)) = (
            self.city_table.get(&from_id).cloned(),
            self.city_table.get(&path[0]).cloned(),
        ) {
            edges.push(build_edge(&from, &to, &self.cities_bounding_box, &self.viewport_dimensions));
        }

        edges
    }
}

impl eframe::App for ProgressPlot {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Some(msg) = try_retrieve_message() {
            self.handle_message(&msg);
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(WHITE))
            .show(ctx, |ui| {
                let painter = ui.painter();

                // While solving: current exploring route in blue.
                // After Done: best route in green (replaces live view).
                if self.is_solved {
                    for (from, to) in &self.best_edges {
                        painter.line_segment([*from, *to], Stroke::new(2.5, BEST_EDGE_COLOR));
                    }
                } else {
                    for (from, to) in &self.current_edges {
                        painter.line_segment([*from, *to], Stroke::new(1.5, CURRENT_EDGE_COLOR));
                    }
                }

                // Orange edge from previous city to current city (B&B active step)
                if let (Some(prev_id), Some(curr_id)) = (self.prev_city_id, self.current_city_id) {
                    let prev_pos = self.nodes.iter().find(|n| n.city_id == prev_id).map(|n| n.pos);
                    let curr_pos = self.nodes.iter().find(|n| n.city_id == curr_id).map(|n| n.pos);
                    if let (Some(p), Some(c)) = (prev_pos, curr_pos) {
                        painter.line_segment([p, c], Stroke::new(3.0, ACTIVE_EDGE_COLOR));
                    }
                }

                // Layer 4: city nodes (black; current city = red)
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
            });

        // Poll at ~60fps while solver runs; avoids burning a full CPU core.
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
    (Pos2::new(fx as f32, fy as f32), Pos2::new(tx as f32, ty as f32))
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
                y_min = y
            }
            if y > y_max {
                y_max = y
            }
        }
    }

    [x_min as f64, y_min as f64, x_max as f64, y_max as f64]
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
