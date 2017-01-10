extern crate openai;
use openai::*;

struct NullGymMember;
impl GymMember for NullGymMember {
   fn start (&mut self, s: GymShape) -> &mut GymMember { return self }
   fn reward(&mut self, r: gym_reward, d: gym_done) {}
   fn reset(&mut self) {}
   fn close(&mut self) {}
}

fn main() {
   let mut m = NullGymMember{};
   let mut d = Gym::new();
   d.parse_args();
   d.start(m);
}
