use std::collections::hash_map::HashMap;
use std::rc::Rc;

use glium::glutin::{
	self,
	event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
	event_loop::ControlFlow,
	window::WindowId,
};

use crate::window::Window;
use crate::NextUpdate;

/// Returns true of original was replaced by new
fn update_control_flow(original: &mut ControlFlow, new: ControlFlow) -> bool {
	if *original == ControlFlow::Exit {
		return false;
	}
	match new {
		ControlFlow::Exit | ControlFlow::Poll => {
			*original = new;
			return true;
		}
		ControlFlow::WaitUntil(new_time) => match *original {
			ControlFlow::WaitUntil(orig_time) => {
				if new_time < orig_time {
					*original = new;
					return true;
				}
			}
			ControlFlow::Wait => {
				*original = new;
				return true;
			}
			_ => (),
		},
		_ => (),
	}
	false
}

pub struct Application {
	pub event_loop: glutin::event_loop::EventLoop<()>,
	windows: HashMap<WindowId, Rc<Window>>,
	global_handlers: Vec<Box<dyn FnMut(&Event<()>) -> NextUpdate>>,
	at_exit: Option<Box<dyn FnOnce()>>,
}

impl Application {
	pub fn new() -> Application {
		Application {
			event_loop: glutin::event_loop::EventLoop::<()>::new(),
			windows: HashMap::new(),
			global_handlers: Vec::new(),
			at_exit: None,
		}
	}

	pub fn set_at_exit<F: FnOnce() + 'static>(&mut self, fun: Option<F>) {
		match fun {
			Some(fun) => self.at_exit = Some(Box::new(fun)),
			None => self.at_exit = None,
		};
	}

	pub fn register_window(&mut self, window: Rc<Window>) {
		self.windows.insert(window.get_id(), window);
	}

	pub fn add_global_event_handler<F: FnMut(&Event<()>) -> NextUpdate + 'static>(
		&mut self,
		fun: F,
	) {
		self.global_handlers.push(Box::new(fun));
	}

	pub fn start_event_loop(self) -> ! {
		let windows = self.windows;
		let mut at_exit = self.at_exit;
		let mut global_handlers = self.global_handlers;
		let mut close_requested = false;
		let mut control_flow_source = *windows.keys().next().unwrap();
		self.event_loop.run(move |event, _event_loop, control_flow| {
			for handler in global_handlers.iter_mut() {
				update_control_flow(control_flow, handler(&event).into());
			}
			match event {
				Event::WindowEvent { event, window_id } => match event {
					WindowEvent::CloseRequested => {
						close_requested = true;
					}
					WindowEvent::KeyboardInput {
						input:
							KeyboardInput {
								virtual_keycode: Some(VirtualKeyCode::Escape),
								state: ElementState::Pressed,
								..
							},
						..
					} => {
						close_requested = true;
					}
					event => {
						if let WindowEvent::Resized { .. } = event {
							windows.get(&window_id).unwrap().request_redraw();
						}
						windows.get(&window_id).unwrap().process_event(event);
					}
				},
				Event::MainEventsCleared => {
					if !close_requested {
						for (_, window) in windows.iter() {
							if window.redraw_needed() {
								window.request_redraw();
								//event_loop
							}
						}
					}
					if close_requested {
						*control_flow = ControlFlow::Exit;
					}
				}
				Event::RedrawRequested(window_id) => {
					let new_control_flow = windows.get(&window_id).unwrap().redraw().into();
					if control_flow_source == window_id {
						*control_flow = new_control_flow;
					} else if *control_flow != ControlFlow::Exit
						&& update_control_flow(control_flow, new_control_flow)
					{
						control_flow_source = window_id;
					}
				}
				Event::RedrawEventsCleared => {
					if close_requested {
						*control_flow = ControlFlow::Exit;
					}
				}
				_ => (),
			}
			if *control_flow == ControlFlow::Exit {
				if let Some(at_exit) = at_exit.take() {
					at_exit();
				}
			}
		});
	}
}
