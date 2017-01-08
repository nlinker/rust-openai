extern crate rust_openai;
use rust_openai::docker;



fn main() {
   let a = rust_openai::docker::Agent{};
   let mut d = rust_openai::docker::Gym::new();
   d.parse_args();
   d.start(a);
}
