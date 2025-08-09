mod chip8;

use std::sync::{mpsc, Arc};
use std::sync::mpsc::Receiver;
use pixels::{Pixels, SurfaceTexture};
use pixels::wgpu::Color;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};
use crate::chip8::DisplayAction;

struct App {
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    receiver: Receiver<DisplayAction>
}

impl App {
    fn new(receiver: Receiver<DisplayAction>) -> Self {
        App{
            window: None,
            pixels: None,
            receiver,
        }
    }
}

impl<'a> ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(event_loop.create_window(Window::default_attributes()).unwrap());
        self.window = Some(window.clone());
        let texture = SurfaceTexture::new(64, 32, window);
        self.pixels = Some(Pixels::new(64, 32, texture).unwrap());
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            },
            WindowEvent::RedrawRequested =>  {
            }
            _ => (),
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);

    let (sender, receiver) = mpsc::channel();
    let mut app = App::new(receiver);

    std::thread::spawn(move || {
        loop {

        }
    });

    std::thread::spawn(move || {
        loop {

        }
    });

    event_loop.run_app(&mut app).unwrap();
}