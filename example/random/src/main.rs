extern crate openai;
use openai::*;

extern crate rand;

struct RandomGymMember;
impl GymMember for RandomGymMember {
   fn start (&mut self, s: &GymShape, mut t: &GymState) {}
   fn reward(&mut self, r: gym_reward, d: gym_done) {}
   fn tick(&mut self) -> u64 { return (rand::random::<u64>())%6 }
   fn reset(&mut self) {}
   fn close(&mut self) {}
}

fn main() {
   let mut d = Gym::new();
   d.parse_args();
   d.start(|| { RandomGymMember{} });
}
