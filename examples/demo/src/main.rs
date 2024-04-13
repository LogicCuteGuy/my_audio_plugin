use nih_plug::prelude::*;
use demo::DemoPlugin;

fn main() {
    nih_export_standalone::<DemoPlugin>();
}