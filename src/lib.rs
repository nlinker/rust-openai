use std::process::{Command,Stdio};
use std::{thread, time, str};
use std::time::{Duration, Instant};
use std::cmp::{max, min};
use std::fs;
use std::fs::File;
use std::path::Path;
use std::string::String;
use std::io::prelude::*;
use std::io::stdout;
use std::net::{TcpStream, SocketAddr};
use std::sync::mpsc::channel;
use std::env;
use std::iter::repeat;
use std::sync::mpsc;
use std::sync::Arc;
use std::f64;

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
pub struct GymShape {
   action_space: gym_range,
   observation_space: gym_range,
   reward_max: gym_reward,
   reward_min: gym_reward
}
pub struct GymState {
   screen: Vec<u8>
}
pub trait GymMember {
   fn start (&mut self, s: &GymShape, t: &GymState) -> ();
   fn reward (&mut self, gym_reward, gym_done) -> ();
   fn reset (&mut self) -> ();
   fn tick (&mut self) -> ();
   fn close (&mut self) -> ();
}
pub struct GymRemote {
   id: u32,
   fps: u32,
   env_id: String,
   duration: u64,
   record_dst: String,
   vnc: Option<vnc::Client>,
   state: GymState,
   shape: GymShape,
   time: Instant,
   frame: u32
}

pub struct Gym {
   fps: u32,
   env_id: String,
   max_parallel: u32,
   duration: u64,
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

const ATARI_HEIGHT: u32 = 262;
const ATARI_WIDTH: u32 = 160;

impl GymRemote {
   pub fn start<T: GymMember>(&mut self, mut agent: T) {
      self.start_rewarder();
      self.start_vnc();
      self.start_agent(agent);

      for _ in 0..self.duration {
         self.sync();
      }
   }
   pub fn sync(&mut self) -> () {
      self.sync_rewarder();
      self.sync_vnc();
      self.sync_agent();
   }
   pub fn start_agent<T: GymMember>(&mut self, mut agent: T) {
      agent.start(&self.shape, &self.state);
      self.time = Instant::now();
   }
   pub fn start_rewarder(&mut self) {
      let ws_url = &format!("ws://127.0.0.1:{}", 15900+self.id)[..];
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
      //receiver.set_nonblocking(true);
      println!("Recorder websocket is now ready to use at {}", 15900+ self.id); 

      let reset_cmd = RewarderCommand{
         method: "v0.env.reset".to_owned(),
         body: RCBody {
            seed: 1,
            env_id: self.env_id.trim().to_string(),
            fps: self.fps
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
   }
   pub fn start_vnc(&mut self) {
      //connect vnc
      let vnc_addr: SocketAddr = format!("127.0.0.1:{}", 5900+self.id).parse().expect("Unable to parse socket address");
      let stream = match std::net::TcpStream::connect(vnc_addr) {
         Ok(stream) => stream,
         Err(error) => {
            panic!("cannot connect to localhost:{}: {}", 5900+self.id, error);
            std::process::exit(1)
         }
      };
      self.vnc = Some(match vnc::Client::from_tcp_stream(stream, true, |methods| {
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
      });
      println!("Connected to vnc on port: {}", 5900+self.id);
   }
   pub fn sync_agent(&mut self) -> () {
   }
   pub fn sync_rewarder(&mut self) -> () {
      /*
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
      */
   }
   pub fn render_frame(&mut self) {
      let width = self.shape.observation_space[0];
      let height = self.shape.observation_space[1];

      let mut imgbuf = image::ImageBuffer::new( width as u32, height as u32 );

      for x in 0 .. width {
         for y in 0 .. height {
            let left = 3*(y * width + x) as usize;
            let pixels = &self.state.screen;
            imgbuf.put_pixel(x as u32, y as u32, image::Rgb([ pixels[left], pixels[left+1], pixels[left+2] ]));
         }
      }
     
      let ref mut fout = File::create(&Path::new( &format!("mov_out/frame_{}.png", self.frame)[..] )).unwrap();
      let _ = image::ImageRgb8(imgbuf).save(fout, image::PNG);

      self.frame = self.frame + 1;
   }
   pub fn sync_vnc(&mut self) -> () {
      let width = self.shape.observation_space[0];
      let height = self.shape.observation_space[1];

      let pause = 1000 / self.fps;
      std::thread::sleep_ms(pause);
      self.render_frame();

      let mut vnc = self.vnc.as_mut().unwrap();

      vnc.request_update(vnc::Rect { left: 0, top: 0, width: width as u16, height: height as u16}, false).unwrap();
      for event in vnc.poll_iter() {
         use vnc::client::Event;
         match event {
            Event::PutPixels(vnc_rect, ref pixels) => {
               let mut black_screen = true;
               /*
               for x in vnc_rect.left .. min(width as u16, (vnc_rect.left+vnc_rect.width)) {
                  for y in vnc_rect.top .. min(height as u16, (vnc_rect.top+vnc_rect.height)) {
                     let i = x - vnc_rect.left;
                     let j = y - vnc_rect.top;
                     let left = 4*(j * vnc_rect.width + i) as usize;
                     if pixels[left]>20 { black_screen = false }
                     if pixels[left+1]>20 { black_screen = false }
                     if pixels[left+2]>20 { black_screen = false }
                  }
               }

               if !black_screen {
               for x in vnc_rect.left .. min(width as u16, (vnc_rect.left+vnc_rect.width)) {
                  for y in vnc_rect.top .. min(height as u16, (vnc_rect.top+vnc_rect.height)) {
                     let i = x - vnc_rect.left;
                     let j = y - vnc_rect.top;
                     let left = 4*(j * vnc_rect.width + i) as usize;
                     self.state.screen[3*(y*width + x)] = pixels[left+2];
                     self.state.screen[3*(y*width + x)+1] = pixels[left+1];
                     self.state.screen[3*(y*width + x)+2] = pixels[left];
                  }
               }}
               */
            },
            _ => {}
         }
      }
   }
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
         } else if prev == "--duration" {
            self.duration = argument.parse::<u64>().unwrap();
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
   pub fn set_duration(&mut self, num: u64) -> () { self.duration = num }
   pub fn set_record(&mut self, dst: String) -> () { self.record_dst = dst.to_string() }
   pub fn new() -> Gym {
      Gym {
         fps: 10,
         env_id: "gym-core.AirRaid-v0".to_string(),
         max_parallel: 1,
         duration: 1800,
         record_dst: "video.mpg".to_string()
      }
   }
   pub fn remote_prep_container(&mut self, pi: u32) -> () {
      let base_vnc = 5900 + pi;
      let base_rec = 15900 + pi;
      let ecode = Command::new("docker")
         .arg("run")
         .arg("-p").arg( format!("5900:{}", base_vnc) )
         .arg("-p").arg( format!("15900:{}", base_rec) )
         .arg("quay.io/openai/universe.gym-core:0.20.0")
         .spawn();
      print!("spawned docker process at {} / {}", base_vnc, base_rec);

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

      let ten_seconds = time::Duration::from_millis(10000);
      thread::sleep(ten_seconds);

   }
   pub fn remote_prep_recorder(&mut self, pi: u32) {
      for entry in glob("mov_out/*.png").expect("Failed to read glob pattern") {
         match entry {
            Ok(path) => {
               fs::remove_file(path);
            }
            Err(e) => {}
         }
      }
      for entry in glob("*.mpg").expect("Failed to read glob pattern") {
         match entry {
            Ok(path) => {
               fs::remove_file(path);
            }
            Err(e) => {}
         }
      }
      fs::create_dir("mov_out/");
   }
   pub fn start_remote(&mut self, pi: u32) -> GymRemote {
      self.remote_prep_container(pi);
      self.remote_prep_recorder(pi);
      let r = GymRemote {
         id: pi,
         fps: self.fps,
         env_id: format!("{: <99}", self.env_id).clone(),
         duration: self.duration,
         record_dst: self.record_dst.clone(),
         vnc: None,
         state: GymState {
            screen: vec![0; (ATARI_WIDTH * ATARI_HEIGHT * 3) as usize]
         },
         shape: GymShape {
            action_space: vec![10],
            observation_space: vec![ATARI_WIDTH as usize, ATARI_HEIGHT as usize, 3 as usize],
            reward_max : f64::INFINITY,
            reward_min : f64::NEG_INFINITY
         },
         time: Instant::now(),
         frame: 0
      };
      return r;
   }
   pub fn recorder_cleanup(&mut self) -> () {
      Command::new("ffmpeg")
                 .arg("-r").arg("5")
                 .arg("-f").arg("image2")
                 .arg("-i").arg("mov_out/frame_%0d.png")
                 .arg("-r").arg("24")
                 .arg("-pix_fmt").arg("yuv420p")
                 .arg(self.record_dst.clone())
                 .spawn();
   }
   pub fn remote_sanitize(&mut self) -> () {
      Command::new("sh").arg("-c").arg("docker kill $(docker ps -q)").spawn();
   }
   pub fn start<F,T: GymMember>(&mut self, start_agent: F) -> ()
   where F: Fn() -> T + Send + Sync + 'static
   {
      let start_agent = Arc::new(start_agent);
      let mut threads = Vec::new();
      self.remote_sanitize();
      for pi in 0..self.max_parallel {
         let start_agent = start_agent.clone();
         let mut remote = self.start_remote(pi);
         threads.push(std::thread::spawn(move || {
            let agent = start_agent();
            remote.start(agent);
         }));
      }

      for t in threads {
         t.join();
      }

      self.recorder_cleanup();
      self.remote_sanitize();
   }
}

