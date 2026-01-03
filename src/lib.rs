
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

pub mod cronet;
pub mod service;

// Include generated bindings
pub mod cronet_c {
    include!(concat!(env!("OUT_DIR"), "/cronet_bindings.rs"));
}

// Include generated proto code
pub mod cronet_pb {
    include!(concat!(env!("OUT_DIR"), "/cronet.engine.v1.rs"));
}
