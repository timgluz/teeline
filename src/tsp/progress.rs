//use piston::window::WindowSettings;
use piston::event_loop::{Events, EventLoop, EventSettings};
use piston_window::*;

use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use super::route::Route;
use super::{SolverOptions, KDPoint};

pub type PublishChannel = Sender<ProgressMessage>;
pub type ReceiverChannel = Receiver<ProgressMessage>;
pub type PublisherFn = Arc<dyn Fn(ProgressMessage) -> ()>;

type RGBA = [f32; 4];
type RectCoords = [f64; 4];

const WHITE: RGBA = [1.0; 4];
const RED: RGBA = [1.0, 0.0, 0.0, 0.9];
const GREEN: RGBA = [0.0, 1.0, 0.0, 0.9];
const BLUE: RGBA = [0.0, 0.0, 1.0, 0.9];
const GREY: RGBA = [0.7, 0.7, 0.7, 0.9];

const ACTIVE_COLOR: RGBA = RED;
const DISACTIVE_COLOR: RGBA = GREY;
const VISITED_COLOR: RGBA = GREEN;

const FONT_SIZE: u32 = 16;

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

        thread::sleep(Duration::from_millis(10));
    })
}

pub fn build_dummy_publisher(verbose: bool) -> PublisherFn {
    Arc::new(move |msg: ProgressMessage| {
        if verbose {
            println!("DummyPublisher: {:?}", msg);
        }

        thread::sleep(Duration::from_millis(10));
    })
}

#[derive(Debug, Clone)]
struct Node {
    id: usize,
    x: f64,
    y: f64,
    height: f64,
    width: f64,
    color: RGBA
}

impl Node {
    fn new(id: usize,x: f64, y: f64, height: f64, width: f64, color: RGBA) -> Self {
        Node {
            id,
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

        [
            self.x - half_w, self.y - half_h, self.width, self.height
        ]
    }
}

#[derive(Debug, Clone)]
struct Edge {
	from: Node,
	to: Node,
	width: f64,
	color: RGBA
}

impl Edge {
	fn new(from: Node, to: Node, color: RGBA, width: f64) -> Self {
		Edge { from, to, width, color }
	}

	pub fn to_line(&self) -> RectCoords {
		[
			self.from.x, self.from.y,
			self.to.x, self.to.y
		]
	}
}

pub struct ProgressPlot {
    options: SolverOptions,
    nodes: Vec<Box<Node>>,
	edges: Vec<Box<Edge>>,
    height: u32,
    width: u32,
    margin: u32,
}

impl ProgressPlot {
    pub fn new(cities: &[KDPoint], options: SolverOptions) -> Self {
        // TODO: read window settings from CLI
        let width = 1024;
        let height = 1024;

        let mut plot = ProgressPlot {
            options,
            nodes: Vec::new(),
			edges: Vec::new(),
            height: width,
            width: height,
            margin: 10,
        };

        plot.add_cities(cities);

		// TODO: publish path and build path from nodes
		// add fake nodes between node1 , node2

		let from_city = plot.nodes[0].clone();
		let to_city = plot.nodes[1].clone();

		plot.add_edge(*from_city, *to_city, BLUE);
        plot
    }

	fn add_edge(&mut self, from_city: Node, to_city: Node, color: RGBA) {
		let new_edge = Edge::new(from_city, to_city, color, 2.0);
		self.edges.push(Box::new(new_edge));
	}

    pub fn run(&mut self, in_channel: ReceiverChannel) {
        let mut done = false;
        let settings = WindowSettings::new("Teeline - TSP solver", [self.height, self.width])
                                .exit_on_esc(true)
                                .resizable(false);

        let mut window: PistonWindow = settings.build()
            .expect("ProgressPlot: Failed to create window");

        let mut glyphs = window.load_font("./assets/FiraSans-Light.ttf").unwrap();

        while let Some(e) = window.next() {
            // render updates
            window.draw_2d(&e, |ctx, renderer, device| {
                clear(WHITE, renderer);

                // render cities
                for s in &self.nodes {
                    rectangle(s.color, s.to_rect(), ctx.transform, renderer);

                    let text_transformer = ctx.transform.trans(s.x + s.width, s.y + s.height);
                    text(RED, FONT_SIZE, format!("{}", s.id).as_ref(), &mut glyphs, text_transformer, renderer).unwrap();
                }

				// render edges
				for edge in &self.edges {
					line(edge.color, edge.width, edge.to_line(), ctx.transform, renderer);
				}
                // update glyphs before rendering
                glyphs.factory.encoder.flush(device);
            });


            // update state
            match in_channel.recv_timeout(Duration::from_millis(10)) {
                Ok(ProgressMessage::Done) => done = true,
                Ok(msg) => self.update(&msg),
                Err(_) => { done = true }
            }
        }
    }

    fn update(&mut self, msg: &ProgressMessage) {
        match msg {
            ProgressMessage::PathUpdate(route, distance) => {
                self.update_route(route, distance.clone());
            },
            ProgressMessage::CityChange(city_id) => self.highlight_city(city_id.clone()),
            _ => println!("ProgressUpdate: {:?}",  msg)
        }
    }

    fn add_cities(&mut self, cities: &[KDPoint]) {
        let window_dims = self.window_dimensions(cities);

        for city in cities.iter() {
            let x = city.get(0).unwrap_or(-1.0) as f64;
            let y = city.get(1).unwrap_or(-1.0) as f64;

            let (x_v, y_v) = self.point_to_viewport(x, y, window_dims);
            let shape = Node::new(
                city.id,
                x_v,
                y_v,
                10.0,
                10.0,
                DISACTIVE_COLOR
            );

            self.nodes.push(Box::new(shape));
        }
    }

    fn update_route(&mut self, route: &Route, distance: f32) {
        println!("Going to update the route");

        // TODO: draw lines between cities
    }

    fn highlight_city(&mut self, city_id: usize) {
        for city in self.nodes.iter_mut() {
            // mark previous active city as visited
            if city.color == ACTIVE_COLOR {
                city.color = VISITED_COLOR;
            }

            // mark current city as active
            if city.id == city_id {
                city.color = ACTIVE_COLOR;
            }
        }
    }

    fn window_dimensions(&self, cities: &[KDPoint]) -> RectCoords {
        let mut x_min = f32::MAX;
        let mut x_max = f32::MIN;
        let mut y_min = f32::MAX;
        let mut y_max = f32::MIN;

        for city in cities.iter() {
            if let Some(x) = city.get(0) {
                if x < x_min { x_min = x; }
                if x > x_max { x_max = x; }
            }

            if let Some(y) = city.get(1) {
                if y < y_min { y_min = y }
                if y > y_max { y_max = y }
            }
        }

        [x_min as f64, y_min as f64, x_max as f64, y_max as f64]
    }

    // converts EUC2D space into GUI coords [0..self.height, 0..self.width]
    fn point_to_viewport(&self, x: f64, y: f64, window: RectCoords) -> (f64, f64) {
        let x_min = window[0];
        let y_min = window[1];
        let x_max = window[2];
        let y_max = window[3];

        // TODO: research how to scale viewport itself, so we can get rid of margin here
        let x_v = (self.width - self.margin * 2) as f64 * (x - x_min) / (x_max - x_min);
        let y_v = (self.height- self.margin * 2) as f64 * (y - y_min) / (y_max - y_min);

        (x_v + (self.margin as f64), y_v + (self.margin as f64))
    }


    fn update_error(&self, err: String) {
        println!("ProgressError: {:?}", err)
    }
}
