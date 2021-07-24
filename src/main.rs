extern crate image;
extern crate piston_window;

mod casette;

use casette::Casette;

fn main() {
    println!("Hello World");

    let casette = Casette::load("../nes-roms/sample1.nes").unwrap();
    casette.show();
}
