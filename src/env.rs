use std::collections::HashMap;

type gym_action = Vec<usize>;
type gym_reward = f64;
type gym_done = bool;

struct GymEnv<O> {
   action_space: Vec<usize>, // finite discrete n-dimensional space
   observation_space: Vec<usize>, // finite discrete n-dimensional space
   reward_lowest: gym_reward, //minimum reward value obtainable (bad)
   reward_highest: gym_reward, //maximum reward value obtainable (good)
   reset : fn () -> (),
   step : fn () -> (O,gym_reward,gym_done),
   close : fn () -> (),
   configure : fn (cfg: HashMap<String,String>) -> (),
   seed : fn (seed: &[u32])  -> ()
}

