use super::client::Client;
use super::committee::Committee;
use super::server::Server;

// a protocol binds compatible Server, Client, and Committee implementations together
pub trait Protocol {
    type Input;
    type Server: Server;
    type Client: Client<Self::Input, ServerState = <Self::Server as Server>::State>;
    type Committee: Committee<ServerState = <Self::Server as Server>::State>;
}
