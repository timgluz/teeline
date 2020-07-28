use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use super::route::Route;
use super::SolverOptions;
use super::Solvers;

pub type PublishChannel = Sender<ProgressMessage>;
pub type ReceiverChannel = Receiver<ProgressMessage>;
pub type PublisherFn = Arc<dyn Fn(ProgressMessage) -> ()>;

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
    })
}

pub fn build_dummy_publisher(verbose: bool) -> PublisherFn {
    Arc::new(move |msg: ProgressMessage| {
        if verbose {
            println!("DummyPublisher: {:?}", msg);
        }
    })
}

pub struct ProgressPlot {
    solver: Solvers,
    options: SolverOptions,
}

impl ProgressPlot {
    pub fn new(solver: Solvers, options: SolverOptions) -> Self {
        ProgressPlot { solver, options }
    }

    pub fn run(&self, in_channel: ReceiverChannel) {
        let mut done = false;

        loop {
            match in_channel.recv() {
                Ok(ProgressMessage::Done) => done = true,
                Ok(msg) => self.update(&msg),
                Err(_) => self.update_error("Failed to retrieve updates.".to_string()),
            }

            if done {
                break;
            };

            thread::sleep(Duration::from_millis(50));
        }
    }

    fn update(&self, msg: &ProgressMessage) {
        println!("ProgressUpdate: {:?} {:?}", self.solver, msg);
    }

    fn update_error(&self, err: String) {
        println!("ProgressError: {:?} {:?}", self.solver, err)
    }
}
