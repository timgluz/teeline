use piston::window::WindowSettings;
//use piston::event_loop::{EventLoop, EventSettings, Events};
use piston_window::*;

use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use super::route::Route;
use super::KDPoint;

pub type PublishChannel = Sender<ProgressMessage>;
pub type ReceiverChannel = Receiver<ProgressMessage>;
pub type PublisherFn = Arc<dyn Fn(ProgressMessage) -> ()>;

type RGBA = [f32; 4];
type RectCoords = [f64; 4];
type Point2D = (f64, f64);

const WHITE: RGBA = [1.0; 4];
const RED: RGBA = [1.0, 0.0, 0.0, 0.9];
const GREEN: RGBA = [0.0, 1.0, 0.0, 0.9];
const BLUE: RGBA = [0.0, 0.0, 1.0, 0.9];
const GREY: RGBA = [0.7, 0.7, 0.7, 0.9];

const ACTIVE_COLOR: RGBA = RED;
const INACTIVE_COLOR: RGBA = GREY;
const VISITED_COLOR: RGBA = GREEN;

const FONT_SIZE: u32 = 16;

const RENDER_FRQ: u64 = 5;

#[derive(Debug, Clone)]
pub enum ProgressMessage {
    CityChange(usize),
    PathUpdate(Route, f32),
    EpochUpdate(usize),
    Done,
    Restart,
}

pub fn build_publisher(publish_ch: PublishChannel) -> PublisherFn {
    Arc::new(move |msg: ProgressMessage| {
        publish_ch
            .send(msg)
            .expect("Failed to publish progress updates");

        thread::sleep(Duration::from_millis(RENDER_FRQ));
    })
}

pub fn build_dummy_publisher(verbose: bool) -> PublisherFn {
    Arc::new(move |msg: ProgressMessage| {
        if verbose {
            println!("DummyPublisher: {:?}", msg);
        }

        thread::sleep(Duration::from_millis(RENDER_FRQ));
    })
}

trait Renderable {
    fn render(&self, ctx: &Context, renderer: &mut G2d, glyphs: &mut Glyphs);

    fn belongs_to_city(&self, other_city_id: usize) -> bool;

    fn set_color(&mut self, new_color: RGBA);
    fn is_edge(&self) -> bool;
}

#[derive(Debug, Clone)]
struct Node {
    city_id: Option<usize>,
    x: f64,
    y: f64,
    height: f64,
    width: f64,
    color: RGBA,
}

impl Node {
    fn new(city_id: Option<usize>, x: f64, y: f64, height: f64, width: f64, color: RGBA) -> Self {
        Node {
            city_id,
            x,
            y,
            height,
            width,
            color,
        }
    }

    fn to_rect(&self) -> RectCoords {
        let half_h = self.height / 2.0;
        let half_w = self.width / 2.0;

        [self.x - half_w, self.y - half_h, self.width, self.height]
    }
}

impl Renderable for Node {
    fn render(&self, ctx: &Context, renderer: &mut G2d, glyphs: &mut Glyphs) {
        rectangle::Rectangle::new(self.color).draw(
            self.to_rect(),
            &ctx.draw_state,
            ctx.transform,
            renderer,
        );

        TextBox::new(
            format!("id.{:?}", self.city_id.unwrap_or(0)),
            self.x,
            self.y + FONT_SIZE as f64,
            self.color,
            FONT_SIZE,
        )
        .render(ctx, renderer, glyphs);
    }

    fn belongs_to_city(&self, other_city_id: usize) -> bool {
        self.city_id
            .map(|c_id| c_id == other_city_id)
            .unwrap_or(false)
    }

    fn set_color(&mut self, new_color: RGBA) {
        self.color = new_color;
    }

    fn is_edge(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone)]
struct Edge {
    from: Point2D,
    to: Point2D,
    width: f64,
    color: RGBA,
}

impl Edge {
    fn new(from: Point2D, to: Point2D, color: RGBA, width: f64) -> Self {
        Edge {
            from,
            to,
            width,
            color,
        }
    }

    fn to_line(&self) -> RectCoords {
        [self.from.0, self.from.1, self.to.0, self.to.1]
    }
}

impl Renderable for Edge {
    fn render(&self, ctx: &Context, renderer: &mut G2d, _glyphs: &mut Glyphs) {
        line::Line::new(self.color, self.width).draw(
            self.to_line(),
            &ctx.draw_state,
            ctx.transform,
            renderer,
        );
    }

    fn belongs_to_city(&self, _other_city_id: usize) -> bool {
        false
    }

    fn set_color(&mut self, new_color: RGBA) {
        self.color = new_color;
    }

    fn is_edge(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone)]
struct TextBox {
    text: String,
    x: f64,
    y: f64,
    font_size: u32,
    color: RGBA,
}

impl TextBox {
    pub fn new<S: Into<String>>(text: S, x: f64, y: f64, color: RGBA, font_size: u32) -> Self {
        TextBox {
            text: text.into(),
            x,
            y,
            color,
            font_size,
        }
    }
}

impl Renderable for TextBox {
    fn render(&self, ctx: &Context, renderer: &mut G2d, glyphs: &mut Glyphs) {
        text::Text::new_color(self.color, self.font_size)
            .draw(
                self.text.as_ref(),
                glyphs,
                &ctx.draw_state,
                ctx.transform.trans(self.x, self.y),
                renderer,
            )
            .unwrap();
    }

    fn belongs_to_city(&self, _other_city_id: usize) -> bool {
        false
    }

    fn set_color(&mut self, new_color: RGBA) {
        self.color = new_color;
    }

    fn is_edge(&self) -> bool {
        false
    }
}

pub struct ProgressPlot {
    city_table: HashMap<usize, KDPoint>,
    shapes: Vec<Box<dyn Renderable>>,
    viewport_dimensions: ViewportDimensions,
    cities_bounding_box: RectCoords,
}

impl ProgressPlot {
    pub fn new(cities: &[KDPoint], width: f64, height: f64, margin: f64) -> Self {
        let mut plot = ProgressPlot {
            city_table: HashMap::new(),
            shapes: Vec::new(),
            viewport_dimensions: ViewportDimensions::new(width, height, margin),
            cities_bounding_box: cities_bounding_box(&cities),
        };

        plot.add_cities(cities);
        plot.add_nodes();
        plot
    }

    pub fn run(&mut self, in_channel: ReceiverChannel) {
        let settings = WindowSettings::new("Teeline - TSP solver", self.window_size())
            .exit_on_esc(true)
            .resizable(false);

        let mut window: PistonWindow = settings
            .build()
            .expect("ProgressPlot: Failed to create window");

        let mut glyphs = window.load_font("./assets/FiraSans-Light.ttf").unwrap();

        while let Some(e) = window.next() {
            // render updates
            window.draw_2d(&e, |ctx, renderer, device| {
                clear(WHITE, renderer);

                // render shapes
                for shape in &self.shapes {
                    shape.render(&ctx, renderer, &mut glyphs);
                }

                // update glyphs before rendering
                glyphs.factory.encoder.flush(device);
            });

            // update state
            if let Ok(msg) = in_channel.recv_timeout(Duration::from_millis(RENDER_FRQ)) {
                self.update(&msg);
            }
        }
    }

    fn update(&mut self, msg: &ProgressMessage) {
        match msg {
            ProgressMessage::Done => self.add_textbox(TextBox::new("Done", 100.0, 100.0, RED, 24)),
            ProgressMessage::PathUpdate(route, _distance) => {
                self.clean_path();
                self.add_path(route);
            }
            ProgressMessage::CityChange(city_id) => self.highlight_city(city_id.clone()),
            _ => println!("ProgressUpdate: {:?}", msg),
        }
    }

    fn add_textbox(&mut self, textbox: TextBox) {
        self.shapes.push(Box::new(textbox));
    }

    fn add_cities(&mut self, cities: &[KDPoint]) {
        for city in cities.iter() {
            self.city_table.insert(city.id, city.clone());
        }
    }

    fn clean_path(&mut self) {
        self.shapes.retain(|x| !x.is_edge());
    }

    fn add_path(&mut self, route: &Route) {
        let path = route.route().to_vec();
        let mut from_city_id = path[0];

        for to_city_id in path.iter().skip(1) {
            println!("Path from {:?} -> {:?}", from_city_id, to_city_id);

            let from_city = self.city_table.get(&from_city_id).unwrap();
            let to_city = self.city_table.get(&to_city_id).unwrap();

            let new_edge = self.build_edge(&from_city, &to_city, GREY);
            self.shapes.push(Box::new(new_edge));

            from_city_id = to_city_id.clone();
        }

        // connect the last city and the first
        let from_city = self.city_table.get(&from_city_id).unwrap();
        let to_city = self.city_table.get(&path[0]).unwrap();

        let new_edge = self.build_edge(&from_city, &to_city, GREY);
        self.shapes.push(Box::new(new_edge));
    }

    fn build_edge(&self, from_city: &KDPoint, to_city: &KDPoint, color: RGBA) -> Edge {
        let from_point = scaled_point(
            from_city,
            &self.cities_bounding_box,
            &self.viewport_dimensions,
        );
        let to_point = scaled_point(
            to_city,
            &self.cities_bounding_box,
            &self.viewport_dimensions,
        );

        Edge::new(from_point, to_point, color, 2.0)
    }

    fn add_nodes(&mut self) {
        for city in self.city_table.values() {
            let city_pt = scaled_point(city, &self.cities_bounding_box, &self.viewport_dimensions);
            let shape = Node::new(
                Some(city.id),
                city_pt.0,
                city_pt.1,
                10.0,
                10.0,
                INACTIVE_COLOR,
            );

            self.shapes.push(Box::new(shape));
        }
    }

    fn highlight_city(&mut self, city_id: usize) {
        for shape in self.shapes.iter_mut() {
            if shape.belongs_to_city(city_id) {
                shape.set_color(ACTIVE_COLOR);
            }
        }
    }

    fn update_error(&self, err: String) {
        println!("ProgressError: {:?}", err)
    }

    fn window_size(&self) -> [f64; 2] {
        [
            self.viewport_dimensions.width,
            self.viewport_dimensions.height,
        ]
    }
}

// -- helper functions

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
    let x_v = (viewport.width - viewport.margin * 2.0) as f64 * (x - x_min) / (x_max - x_min);
    let y_v = (viewport.height - viewport.margin * 2.0) as f64 * (y - y_min) / (y_max - y_min);

    (
        x_v + (viewport.margin as f64),
        y_v + (viewport.margin as f64),
    )
}
