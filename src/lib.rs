use std::process::Command;
use std::{thread, time, str};
use std::string::String;
use std::io::prelude::*;
use std::io::stdin;
use std::net::TcpStream;
use std::sync::mpsc::channel;
use std::env;

extern crate hyper;
use self::hyper::header::{Headers, Authorization, Basic};

extern crate websocket;
use self::websocket::{Message, Sender, Receiver};
use self::websocket::client::request::Url;
use self::websocket::client::Request;
use self::websocket::Client;
use self::websocket::header::extensions::Extension;

use std::collections::HashMap;

type gym_reward = f64;
type gym_done = bool;
type gym_range = Vec<usize>;
type gym_point = Vec<u64>;
pub struct GymShape {
   action_space: gym_range,
   observation_space: gym_range,
   reward_max: gym_reward,
   reward_min: gym_reward
}
pub struct GymEnv {
   look : fn() -> gym_point,
   cause : fn(a: gym_point) -> (),
   reset : fn () -> (),
   close : fn () -> ()
}
pub struct GymMember {
   start : fn (GymShape) -> (),
   reward : fn (gym_reward,gym_done) -> (),
   reset : fn () -> (),
   close : fn () -> ()
}

pub struct Gym {
   fps: u32,
   env_id: String,
   max_parallel: u32,
   num_games: u32,
   record_dst: String
}
impl Gym {
   pub fn parse_args(&mut self) -> () {
      let mut prev = "".to_string();
      for argument in std::env::args() {
         if prev == "--fps" {
            self.fps = argument.parse::<u32>().unwrap();
            prev = "".to_string();
         } else if prev == "--game" {
            self.env_id = argument;
            prev = "".to_string();
         } else if prev == "--parallel" {
            self.max_parallel = argument.parse::<u32>().unwrap();
            prev = "".to_string();
         } else if prev == "--num_games" {
            self.num_games = argument.parse::<u32>().unwrap();
            prev = "".to_string();
         } else if prev == "--record" {
            self.record_dst = argument;
            prev = "".to_string();
         } else {
            prev = argument;
         }
      }
   }
   pub fn set_fps(&mut self, fps: u32) -> () { self.fps = fps }
   pub fn set_game(&mut self, game: String) -> () { self.env_id = game.to_string() }
   pub fn set_max_parallel(&mut self, par: u32) -> () { self.max_parallel = par }
   pub fn set_num_games(&mut self, num: u32) -> () { self.num_games = num }
   pub fn set_record(&mut self, dst: String) -> () { self.record_dst = dst.to_string() }
   pub fn new() -> Gym {
      Gym {
         fps: 60,
         env_id: "".to_string(),
         max_parallel: 1,
         num_games: 1,
         record_dst: "".to_string()
      }
   }
   pub fn start(&mut self, agent: GymMember) -> () {

      for pi in 0..self.max_parallel {
         let base_vnc = 5900 + pi;
         let base_rec = 15900 + pi;
         let ecode = Command::new("docker")
                 .arg("run")
                 .arg("-p").arg( format!("5900:{}", base_vnc) )
                 .arg("-p").arg( format!("15900:{}", base_rec) )
                 .arg("quay.io/openai/universe.gym-core:0.20.0")
                 .status()
                 .expect("failed to execute docker run. Is docker installed?");
         if ecode.success() {
             print!("spawned docker process at {} / {}", base_vnc, base_rec);
         } else {
             panic!("Unable to spawn docker process at {} / {}. Have you checked for zombies?", base_vnc, base_rec);
         }
      }

      //TODO
      //connect to vnc and rewarder
      //start playing
      //start recording results to movie
      //wait
      //stop
      //cleanup dockers etc.
   }
}

pub fn spawn_env() {

   let url = Url::parse("ws://127.0.0.1:15900").unwrap();
   println!("Connecting to {}", url);

   let mut request = Client::connect(url).unwrap();

   let mut auth_header = self::hyper::header::Authorization(
      self::hyper::header::Basic {
         username: "openai".to_owned(),
         password: Some("openai".to_owned())
      }
   );
   request.headers.set(auth_header);
   let mut response = request.send().unwrap(); // Send the request and retrieve a response
   println!("Validating response...");

   response.validate().unwrap(); // Validate the response
   println!("Successfully connected");

   let (mut sender, mut receiver) = response.begin().split();
   let (tx, rx) = channel();
   let tx_1 = tx.clone();

   let send_loop = thread::spawn(move || {
      loop {
         // Send loop
         let message: Message = match rx.recv() {
            Ok(m) => m,
            Err(e) => {
               println!("Send Loop: {:?}", e);
               return;
            }
         };
         match message.opcode {
            websocket::message::Type::Close => {
               let _ = sender.send_message(&message);
               // If it's a close message, just send it and then return.
               return;
            },
            _ => (),
          }
          // Send the message
          match sender.send_message(&message) {
             Ok(()) => (),
             Err(e) => {
                println!("Send Loop: {:?}", e);
                let _ = sender.send_message(&Message::close());
                return;
             }
          }
       }
   });

   let receive_loop = thread::spawn(move || {
      // Receive loop
      for message in receiver.incoming_messages() {
         let message: Message = match message {
            Ok(m) => m,
            Err(e) => {
               println!("Receive Loop: {:?}", e);
               let _ = tx_1.send(Message::close());
               return;
            }
         };
         match message.opcode {
            websocket::message::Type::Close => {
               // Got a close message, so send a close message and return
               let _ = tx_1.send(Message::close());
               return;
            }
            websocket::message::Type::Ping => match tx_1.send(Message::pong(message.payload)) {
               // Send a pong in response
               Ok(()) => (),
               Err(e) => {
                  println!("Receive Loop: {:?}", e);
                  return;
               }
            },
            websocket::message::Type::Text => {
               let bytes = message.payload.into_owned();
               let msg = String::from_utf8(bytes).unwrap();
               println!("Received message from rewarder: {}", msg);
            },
            // Say what we received
            _ => println!("Receive Loop: {:?}", message),
         }
      }
   });

   
   let reset_cmd = "{\"method\":\"v0.env.reset\",\"body\":{\"seed\":1,\"env_id\":\"gym-core.AirRaid-v0\",\"fps\":60},\"headers\":{\"sent_at\":0,\"episode_id\":0,\"message_id\":0}}";
   let message = Message::text(String::from(reset_cmd));
   match tx.send(message) {
      Ok(()) => (),
      Err(e) => {
         println!("Main Loop: {:?}", e);
      }
   }

   // We're exiting
   println!("Waiting for child threads to exit");
   let _ = send_loop.join();
   let _ = receive_loop.join();
   println!("Exited");

}
