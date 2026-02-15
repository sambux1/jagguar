use std::collections::HashSet;

pub struct PortPool {
	start_port: u16,
	max_ports: usize,
	in_use: HashSet<u16>,
}

impl PortPool {
	pub fn new(start_port: u16, max_ports: usize) -> Self {
		Self {
			start_port,
			max_ports,
			in_use: HashSet::new(),
		}
	}

	pub fn is_available(&self, port: u16) -> bool {
		self.is_in_range(port) && !self.in_use.contains(&port)
	}

	pub fn allocate_port(&mut self) -> Option<u16> {
		for i in 0..self.max_ports {
			let port = self.start_port + i as u16;
			if !self.in_use.contains(&port) {
				self.in_use.insert(port);
				return Some(port);
			}
		}
		None
	}

	pub fn release_port(&mut self, port: u16) -> bool {
		self.in_use.remove(&port)
	}

	fn is_in_range(&self, port: u16) -> bool {
		port >= self.start_port && port < self.start_port + self.max_ports as u16
	}
}
