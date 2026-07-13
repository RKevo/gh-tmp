use godot::prelude::*;

pub mod classes;

struct Extry;
pub type Real = f32;


#[gdextension]
unsafe impl ExtensionLibrary for Extry {}
