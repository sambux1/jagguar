use std::collections::HashSet;

/// First-fit pool of localhost ports.
pub struct PortPool {
    start_port: u16,
    max_ports: usize,
    in_use: HashSet<u16>,
}

impl PortPool {
    /// Ports `[start_port, start_port + max_ports)`.
    pub fn new(start_port: u16, max_ports: usize) -> Self {
        Self {
            start_port,
            max_ports,
            in_use: HashSet::new(),
        }
    }

    /// In range and not currently allocated.
    pub fn is_available(&self, port: u16) -> bool {
        self.is_in_range(port) && !self.in_use.contains(&port)
    }

    /// Lowest free port in the range, or `None` if the pool is exhausted.
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

    /// Mark `port` free. Returns whether it was allocated.
    pub fn release_port(&mut self, port: u16) -> bool {
        self.in_use.remove(&port)
    }

    /// Whether `port` is in `[start_port, start_port + max_ports)`.
    fn is_in_range(&self, port: u16) -> bool {
        port >= self.start_port && port < self.start_port + self.max_ports as u16
    }
}
