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

const WHITE: Color32 = Color32::from_rgb(255, 255, 255);
// Premultiplied: rgb * (alpha/255). Alpha ≈ 230 (≈90% opacity).
const RED: Color32 = Color32::from_rgba_premultiplied(230, 0, 0, 230);
const GREY: Color32 = Color32::from_rgba_premultiplied(162, 162, 162, 230);
const INACTIVE_COLOR: Color32 = GREY;
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
    color: Color32,
}

type EdgeData = (Pos2, Pos2);

pub struct ProgressPlot {
    city_table: HashMap<usize, KDPoint>,
    nodes: Vec<NodeData>,
    edges: Vec<EdgeData>,
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
            edges: Vec::new(),
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
                self.status = "Done".to_string();
            }
            ProgressMessage::PathUpdate(route, distance) => {
                self.status = format!("Solving... | best: {:.2}", distance);
                self.edges.clear();
                self.add_path(route);
            }
            ProgressMessage::CityChange(_) => {
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
                color: INACTIVE_COLOR,
            });
        }
    }

    fn add_path(&mut self, route: &Route) {
        let path = route.route().to_vec();
        let mut from_city_id = path[0];

        for to_city_id in path.iter().skip(1) {
            let from_city = self.city_table.get(&from_city_id).cloned();
            let to_city = self.city_table.get(to_city_id).cloned();

            if let (Some(from), Some(to)) = (from_city, to_city) {
                let edge = build_edge(&from, &to, &self.cities_bounding_box, &self.viewport_dimensions);
                self.edges.push(edge);
                from_city_id = *to_city_id;
            }
        }

        if let (Some(from), Some(to)) = (
            self.city_table.get(&from_city_id).cloned(),
            self.city_table.get(&path[0]).cloned(),
        ) {
            let edge = build_edge(&from, &to, &self.cities_bounding_box, &self.viewport_dimensions);
            self.edges.push(edge);
        }
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

                for (from, to) in &self.edges {
                    painter.line_segment([*from, *to], Stroke::new(2.0, GREY));
                }

                for node in &self.nodes {
                    painter.rect_filled(
                        Rect::from_center_size(node.pos, egui::Vec2::splat(10.0)),
                        0.0,
                        node.color,
                    );
                    painter.text(
                        Pos2::new(node.pos.x, node.pos.y + FONT_SIZE),
                        Align2::LEFT_TOP,
                        format!("id.{}", node.city_id),
                        FontId::proportional(FONT_SIZE),
                        node.color,
                    );
                }

                let margin = self.viewport_dimensions.margin as f32;
                painter.text(
                    Pos2::new(margin, margin / 2.0),
                    Align2::LEFT_TOP,
                    &self.status,
                    FontId::proportional(FONT_SIZE),
                    RED,
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
