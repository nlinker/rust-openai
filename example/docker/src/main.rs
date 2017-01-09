extern crate rust_openai;

struct NullGymMember;
impl rust_openai::GymMember for NullGymMember {
   fn start (&mut self, s: rust_openai::GymShape) -> &mut rust_openai::GymMember { return self }
   fn reward(&mut self, r: rust_openai::gym_reward, d: rust_openai::gym_done) {}
   fn reset(&mut self) {}
   fn close(&mut self) {}
}

fn main() {
   let mut m = NullGymMember{};
   let mut d = rust_openai::Gym::new();
   d.parse_args();
   d.start(m);
}
