use std::process::Command;
use std::{thread, time, str};
use std::fs;
use std::fs::File;
use std::path::Path;
use std::string::String;
use std::io::prelude::*;
use std::io::stdin;
use std::net::{TcpStream, SocketAddr};
use std::sync::mpsc::channel;
use std::env;
use std::hash::{Hash, SipHasher, Hasher};

extern crate glob;
use glob::glob;

extern crate rand;
extern crate x11;
extern crate vnc;
extern crate image;

extern crate rustc_serialize;
use rustc_serialize::json;

extern crate hyper;
use self::hyper::header::{Headers, Authorization, Basic};

extern crate websocket;
use self::websocket::{Message, Sender, Receiver};
use self::websocket::client::request::Url;
use self::websocket::client::Request;
use self::websocket::Client;
use self::websocket::header::extensions::Extension;

use std::collections::HashMap;

pub type gym_reward = f64;
pub type gym_done = bool;
pub type gym_range = Vec<usize>;
pub type gym_point = Vec<u64>;
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
pub trait GymMember {
   fn start (&mut self, GymShape) -> (&mut GymMember);
   fn reward (&mut self, gym_reward,gym_done) -> ();
   fn reset (&mut self) -> ();
   fn close (&mut self) -> ();
}

pub struct Gym {
   fps: u32,
   env_id: String,
   max_parallel: u32,
   num_games: u32,
   record_dst: String
}

#[derive(RustcDecodable, RustcEncodable)]
struct RCBody {
   seed: u32,
   env_id: String,
   fps: u32
}

#[derive(RustcDecodable, RustcEncodable)]
struct RCHeaders {
   sent_at: u32,
   episode_id: u32,
   message_id: u32
}

#[derive(RustcDecodable, RustcEncodable)]
struct RewarderCommand {
   method: String,
   body: RCBody,
   headers: RCHeaders
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
         fps: 10,
         env_id: "gym-core.AirRaid-v0".to_string(),
         max_parallel: 1,
         num_games: 1,
         record_dst: "".to_string()
      }
   }
   pub fn start<T: GymMember>(&mut self, agent: T) -> () {

      for pi in 0..self.max_parallel {
         let base_vnc = 5900 + pi;
         let base_rec = 15900 + pi;
         let ecode = Command::new("docker")
                 .arg("run")
                 .arg("-p").arg( format!("5900:{}", base_vnc) )
                 .arg("-p").arg( format!("15900:{}", base_rec) )
                 .arg("quay.io/openai/universe.gym-core:0.20.0")
                 .spawn();
         print!("spawned docker process at {} / {}", base_vnc, base_rec);
      }

      for pi in 0..self.max_parallel {
         let mut ok = false;
         for _ in 0..15 { //wait up to 15 seconds for dockers to boot
            if !ok {
               let rec_port = 15900 + pi;

               println!("Polling docker at port {} for connectivity.", rec_port);
               let one_second = time::Duration::from_millis(1000);
               thread::sleep(one_second);

               let ns1: SocketAddr = format!("127.0.0.1:{}", rec_port).parse().expect("Unable to parse socket address");
               let mut s1 = TcpStream::connect(ns1);

               match s1 {
                  Ok(_) => { ok=true; }
                  _ => { println!("No connectivity to {}", rec_port); }
               }
            }
         }
         if !ok { panic!("Unable to confirm connectivity to docker #{}", pi) }
         else { println!("Confirmed connectivity to docker #{}", pi); }
      }
      let five_seconds = time::Duration::from_millis(5000);
      thread::sleep(five_seconds);

      let mut threads = Vec::new();
      for pi in 0..self.max_parallel {
         let self_env_id = format!("{: <99}", self.env_id).clone();
         let self_fps = self.fps;

         threads.push(thread::spawn(move || {

            let ws_url = &format!("ws://127.0.0.1:{}", 15900+pi)[..];
            let url = Url::parse(ws_url).unwrap();
            println!("Connecting to rewarder at {}", url);
            let mut request = Client::connect(url).unwrap();

            let mut auth_header = self::hyper::header::Authorization(
               self::hyper::header::Basic {
                  username: "openai".to_owned(),
                 password: Some("openai".to_owned())
               }
            );

            request.headers.set(auth_header);
            let mut response = request.send().unwrap(); // Send the request and retrieve a response
            response.validate().unwrap(); // Validate the response
            let (mut sender, mut receiver) = response.begin().split();

            // websocket receiver nonblocking is bugged, so we'll just put it in it's own thread...
            // receiver.set_nonblocking(true);
            println!("Recorder websocket is now ready to use at {}", 15900+pi); 

            //TODO
            //connect to vnc and rewarder
            //start playing
            //start recording results to movie

            let reset_cmd = RewarderCommand{
               method: "v0.env.reset".to_owned(),
               body: RCBody {
                  seed: 1,
                  env_id: self_env_id.trim().to_string(),
                  fps: self_fps
               },
               headers: RCHeaders {
                  sent_at: 0,
                  episode_id: 0,
                  message_id: 0
               }
            };
            let reset_msg = json::encode(&reset_cmd).unwrap();
            println!("Send reset json message to rewarder: {}", reset_msg);
            let rst_msg = Message::text(String::from(reset_msg));
            sender.send_message(&rst_msg);

            //connect vnc
            let vnc_addr: SocketAddr = format!("127.0.0.1:{}", 5900+pi).parse().expect("Unable to parse socket address");
            let stream = match std::net::TcpStream::connect(vnc_addr) {
               Ok(stream) => stream,
               Err(error) => {
                  panic!("cannot connect to localhost:{}: {}", 5900+pi, error);
                  std::process::exit(1)
               }
            };
            let mut vnc = match vnc::Client::from_tcp_stream(stream, true, |methods| {
               for method in methods {
                  match method {
                     &vnc::client::AuthMethod::Password => {
                        let mut key = [0; 8];
                        for (i, byte) in "openai".bytes().enumerate() {
                           if i == 8 { break }
                           key[i] = byte
                        }
                        return Some(vnc::client::AuthChoice::Password(key))
                     }
                     _ => ()
                  }
               }
               None
            }) {
               Ok(vnc) => vnc,
               Err(error) => {
                  panic!("cannot initialize VNC session: {}", error);
                  std::process::exit(1)
               }
            };
            println!("Connected to vnc on port: {}", 5900+pi);

            //let ws have it's own thread because it doesn't play well with others
            thread::spawn(move || {
               for msg in receiver.incoming_messages() {
                  let msg: Result<websocket::message::Message,_> = msg;
                  match msg {
                     Ok(message) => {
                        match message.opcode {
                           websocket::message::Type::Close => {
                              //agent.close()
                              println!("Connection to rewarder terminated.");
                           },
                           websocket::message::Type::Ping => { 
                              let mut pong_message = Message::pong(message.payload);
                              sender.send_message(&pong_message);
                           },
                           websocket::message::Type::Text => {
                             let bytes = message.payload.into_owned();
                             let msg = String::from_utf8(bytes).unwrap();
                             println!("Received message from rewarder: {}", msg);
                           },
                           _ => {}
                        }
                     }
                     Err(e) => {
                        //if no frames available, IOError occurs
                     }
                  }
               }
            });


            let (mut width, mut height) = vnc.size();
            let mut screen = image::ImageBuffer::new(width as u32, height as u32);
            for entry in glob("mov_out/*.png").expect("Failed to read glob pattern") {
               match entry {
                  Ok(path) => {
                    fs::remove_file(path);
                  }
                  Err(e) => {}
               }
            }
            fs::create_dir("mov_out/");

            let mut frame_i = 0;
            loop {
               frame_i = frame_i + 1;
               //let agent = agent.start();
               //TODO, update screen view

               use x11::keysym::*;
               std::thread::sleep_ms(100);
               vnc.request_update(vnc::Rect { left: 0, top: 0, width: width, height: height}, false).unwrap();

               if rand::random() {
                  vnc.send_key_event(false, XK_Right).unwrap();
                  vnc.send_key_event(true, XK_Left).unwrap();
               } else { 
                  vnc.send_key_event(false, XK_Left).unwrap();
                  vnc.send_key_event(true, XK_Right).unwrap();
               }
               
               //poll vnc connection for updates
               for event in vnc.poll_iter() {
                  use vnc::client::Event;

                  match event {
                     Event::Disconnected(_) => {
                        println!("Disconnected Event")
                     },
                     Event::Resize(new_width, new_height) => {
                        println!("Resize Event")
                     },
                     Event::PutPixels(vnc_rect, ref pixels) => {
                        let mut s = SipHasher::new();
                        pixels.hash(&mut s);
                        println!("Pixel Hash: {}", s.finish());
                        for x in vnc_rect.left .. (vnc_rect.left+vnc_rect.width) {
                           for y in vnc_rect.top .. (vnc_rect.top+vnc_rect.height) {
                              let i = x - vnc_rect.left;
                              let j = y - vnc_rect.top;
                              let left = 4*(j * vnc_rect.width + i) as usize;
                              screen.put_pixel(x as u32, y as u32, image::Rgb([ pixels[left+2], pixels[left+1], pixels[left] ]));
                           }
                        }
                     },
                     Event::CopyPixels { src: vnc_src, dst: vnc_dst } => {
                        println!("CopyPixels Event")
                     },
                     Event::EndOfFrame => {
                     },
                     Event::Clipboard(ref text) => {
                       println!("Clipboard Event")
                     },
                     Event::SetCursor {
                        size:    (width, height),
                        hotspot: (new_hotspot_x, new_hotspot_y),
                        pixels,
                        mask_bits
                     } => {
                        println!("received SetCursor Event");
                     }
                     _ => {
                        println!("Received some other message from vnc");
                     }
                  }
               }
               
               let ref mut fout = File::create(&Path::new( &format!("mov_out/frame_{}.png", frame_i)[..] )).unwrap();
               let _ = image::ImageRgb8(screen.clone()).save(fout, image::PNG);
            }
         }));
      }

      for t in threads {
         t.join();
      }
      println!("All remotes terminated. Will now cleanup remote resources.");
      //TODO
      //cleanup dockers etc.
   }
}

