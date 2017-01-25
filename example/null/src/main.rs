extern crate openai;
use openai::*;

struct NullGymMember;
impl GymMember for NullGymMember {
   fn start (&mut self, s: &GymShape, mut t: &GymState) {}
   fn reward(&mut self, r: gym_reward, d: gym_done) {}
   fn tick(&mut self) {}
   fn reset(&mut self) {}
   fn close(&mut self) {}
}

fn main() {
   let mut d = Gym::new();
   d.parse_args();
   d.start(|| { NullGymMember{} });
}
