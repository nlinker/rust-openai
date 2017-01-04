extern crate rust_openai;
use rust_openai::docker;

fn main() {
   rust_openai::docker::spawn_env();
}
