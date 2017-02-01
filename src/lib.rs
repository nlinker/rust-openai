use std::process::{Command,Stdio};
use std::{thread, time, str};
use std::time::{Duration, Instant};
use std::cmp::{max, min};
use std::fs;
use std::fs::File;
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
use std::ptr;
use std::mem;
use std::iter;
use std::path::{Path, PathBuf};
use std::ffi::CString;
use std::iter::FromIterator;

extern crate libc;
use libc::c_void;

extern crate ffmpeg_sys;
use ffmpeg_sys::{SwsContext, AVCodec, AVCodecContext, AVPacket, AVFormatContext, AVStream,
                 AVFrame, AVRational, AVPixelFormat, AVPicture, AVCodecID};


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
use self::websocket::{Message, Sender, Receiver, Client};
use self::websocket::client::request::Url;
use self::websocket::client;
use self::websocket::client::Request;
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
   fn tick (&mut self) -> u64;
   fn close (&mut self) -> ();
}
pub struct GymRemote {
   id: u32,
   fps: u32,
   env_id: String,
   duration: u64,
   record_dst: String,
   mode: String,
   vnc: Option<vnc::Client>,
   rewarder: Option<((self::websocket::client::Sender<websocket::WebSocketStream>, self::websocket::client::Receiver<websocket::WebSocketStream>))>,
   state: GymState,
   shape: GymShape,
   time: Instant,
   frame: u32
}
pub struct MpegEncoder {
    frame_format: i32,
    frame_width: i32,
    frame_height: i32,
    frame_pts: i32,
    tmp_frame_buf:    Vec<u8>,
    frame_buf:        Vec<u8>,
    curr_frame_index: usize,
    bit_rate:         usize,
    target_width:     usize,
    target_height:    usize,
    time_base:        (usize, usize),
    gop_size:         usize,
    max_b_frames:     usize,
    pix_fmt:          AVPixelFormat,
    tmp_frame:        *mut AVPicture,
    frame:            *mut AVPicture,
    context:          *mut AVCodecContext,
    format_context:   *mut AVFormatContext,
    video_st:         *mut AVStream,
    scale_context:    *mut SwsContext,
    path:             PathBuf
}
impl MpegEncoder {
   pub fn new(path: String, width: usize, height: usize) -> MpegEncoder {
      let mut pathbuf = PathBuf::new(); pathbuf.push(path);;

      let bit_rate     = 400000;
      let time_base    = (1, 60);
      let gop_size     = 10;
      let max_b_frames = 1;
      let pix_fmt      = AVPixelFormat::AV_PIX_FMT_YUV420P;
      let width        = if width  % 2 == 0 { width }  else { width + 1 };
      let height       = if height % 2 == 0 { height } else { height + 1 };

      return MpegEncoder {
            frame_format:     0,
            frame_width:      0,
            frame_height:     0,
            frame_pts:        0,
            curr_frame_index: 0,
            bit_rate:         bit_rate,
            target_width:     width,
            target_height:    height,
            time_base:        time_base,
            gop_size:         gop_size,
            max_b_frames:     max_b_frames,
            pix_fmt:          pix_fmt,
            frame:            ptr::null_mut(),
            tmp_frame:        ptr::null_mut(),
            context:          ptr::null_mut(),
            scale_context:    ptr::null_mut(),
            format_context:   ptr::null_mut(),
            video_st:         ptr::null_mut(),
            path:             pathbuf,
            frame_buf:        Vec::new(),
            tmp_frame_buf:    Vec::new()
      }
   }
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
   pub fn recorder_cleanup(&mut self, mp: MpegEncoder) -> () {
      let real_fps = self.frame / max(self.time.elapsed().as_secs() as u32, 1);

            /*

                  // Get the delayed frames.
            let mut pkt: AVPacket = unsafe { mem::uninitialized() };
            let mut got_output = 1;
            while got_output != 0 {
                let ret;

                unsafe {
                    ffmpeg_sys::av_init_packet(&mut pkt);
                }

                pkt.data = ptr::null_mut();  // packet data will be allocated by the encoder
                pkt.size = 0;

                unsafe {
                    ret = ffmpeg_sys::avcodec_encode_video2(mp.context, &mut pkt, ptr::null(), &mut got_output);
                }

                if ret < 0 {
                    panic!("Error encoding frame.");
                }

                if got_output != 0 {
                    unsafe {
                        let _ = ffmpeg_sys::av_interleaved_write_frame(mp.format_context, &mut pkt);
                        ffmpeg_sys::av_free_packet(&mut pkt);
                    }
                }
            }
            // alloc the buffer
            let reps = iter::repeat(0u8).take(nframe_bytes as usize);
            */

            // Free things and stuffs.
            unsafe {
                let _ = ffmpeg_sys::avcodec_close(mp.context);
                ffmpeg_sys::av_free(mp.context as *mut c_void);
            }
   }
   pub fn start<T: GymMember>(&mut self, mut agent: T) {
      let mut capture = self.start_recorder();
      self.start_rewarder();
      self.start_vnc();
      let mut agent = self.start_agent(agent);

      loop {
         use x11::keysym::*;
         if self.time.elapsed().as_secs() > self.duration { break; }
         self.sync();
         let mut vnc = self.vnc.as_mut().unwrap();

         let action = agent.tick();
         if self.mode=="atari" {
            if action==0 {
               vnc.send_key_event(false, XK_Left).unwrap();
               vnc.send_key_event(false, XK_Right).unwrap();
               vnc.send_key_event(false, XK_Up).unwrap();
               vnc.send_key_event(false, XK_Down).unwrap();
               vnc.send_key_event(false, XK_space).unwrap();
            } else if action==1 {
               vnc.send_key_event(true, XK_Left).unwrap();
            } else if action==2 {
               vnc.send_key_event(true, XK_Right).unwrap();
            } else if action==3 {
               vnc.send_key_event(true, XK_Up).unwrap();
            } else if action==4 {
               vnc.send_key_event(true, XK_Down).unwrap();
            } else if action==5 {
               vnc.send_key_event(true, XK_space).unwrap();
            }
         }
      }
      self.recorder_cleanup(capture);
   }
   pub fn sync(&mut self) {
      self.sync_rewarder();
      self.sync_vnc();
   }
   pub fn start_agent<T: GymMember>(&mut self, mut agent: T) -> T {
      agent.start(&self.shape, &self.state);
      self.time = Instant::now();
      return agent
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

      //ws library not updated to use nonblocking yet
      //sender.set_nonblocking(true); 
      //receiver.set_nonblocking(true);

      // websocket receiver nonblocking is bugged, so we'll just put it in it's own thread...
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

      //self.rewarder = Some((sender,receiver));

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
   pub fn start_recorder(&mut self) -> MpegEncoder {
      let width = self.shape.observation_space[0];
      let height = self.shape.observation_space[1];
      let name = "video.mpg".to_string();

      let mut mp = MpegEncoder::new(name, width, height);

      unsafe {
         ffmpeg_sys::av_register_all();
      }

      let path_str = CString::new(mp.path.to_str().unwrap()).unwrap();
      unsafe {
            // try to guess the container type from the path.
            let mut fmt = ptr::null_mut();
            

            let _ = ffmpeg_sys::avformat_alloc_output_context2(&mut fmt, ptr::null_mut(), ptr::null(), path_str.as_ptr());

            if mp.format_context.is_null() {
                // could not guess, default to MPEG
                let mpeg = CString::new(&b"mpeg"[..]).unwrap();
                
                let _ = ffmpeg_sys::avformat_alloc_output_context2(&mut fmt, ptr::null_mut(), mpeg.as_ptr(), path_str.as_ptr());
            }

            mp.format_context = fmt;

            if mp.format_context.is_null() {
                panic!("Unable to create the output context.");
            }

            let fmt = (*mp.format_context).oformat;

            if (*fmt).video_codec == AVCodecID::AV_CODEC_ID_NONE {
                panic!("The selected output container does not support video encoding.")
            }

            let codec: *mut AVCodec;

            let ret: i32 = 0;

            codec = ffmpeg_sys::avcodec_find_encoder((*fmt).video_codec);

            if codec.is_null() {
                panic!("Codec not found.");
            }

            mp.video_st = ffmpeg_sys::avformat_new_stream(mp.format_context, codec);

            if mp.video_st.is_null() {
                panic!("Failed to allocate the video stream.");
            }

            (*mp.video_st).id = ((*mp.format_context).nb_streams - 1) as i32;

            mp.context = (*mp.video_st).codec;

            let _ = ffmpeg_sys::avcodec_get_context_defaults3(mp.context, codec);

            if mp.context.is_null() {
                panic!("Could not allocate video codec context.");
            }


            // sws scaling context
            mp.scale_context = ffmpeg_sys::sws_getContext(
                mp.target_width as i32, mp.target_height as i32, AVPixelFormat::AV_PIX_FMT_RGB24,
                mp.target_width as i32, mp.target_height as i32, mp.pix_fmt,
                ffmpeg_sys::SWS_BICUBIC as i32, ptr::null_mut(), ptr::null_mut(), ptr::null());

            // Put sample parameters.
            (*mp.context).bit_rate = mp.bit_rate as i64;

            // Resolution must be a multiple of two.
            (*mp.context).width    = mp.target_width  as i32;
            (*mp.context).height   = mp.target_height as i32;

            // frames per second.
            let (tnum, tdenum)           = mp.time_base;
            (*mp.context).time_base    = AVRational { num: tnum as i32, den: tdenum as i32 };
            (*mp.video_st).time_base   = (*mp.context).time_base;
            (*mp.context).gop_size     = mp.gop_size as i32;
            (*mp.context).max_b_frames = mp.max_b_frames as i32;
            (*mp.context).pix_fmt      = mp.pix_fmt;

            if (*mp.context).codec_id == AVCodecID::AV_CODEC_ID_MPEG1VIDEO {
                // Needed to avoid using macroblocks in which some coeffs overflow.
                // This does not happen with normal video, it just happens here as
                // the motion of the chroma plane does not match the luma plane.
                (*mp.context).mb_decision = 2;
            }

            if ffmpeg_sys::avcodec_open2(mp.context, codec, ptr::null_mut()) < 0 {
                panic!("Could not open the codec.");
            }

            /*
             * Init the destination video frame.
             */

            mp.frame_format=(*mp.context).pix_fmt as i32;
            mp.frame_width=(*mp.context).width;
            mp.frame_height=(*mp.context).height;
            mp.frame_pts = 0;


            let nframe_bytes = ffmpeg_sys::avpicture_get_size(mp.pix_fmt,
                                                              mp.target_width as i32,
                                                              mp.target_height as i32);


            /* NEXT segfaults here
            let reps = iter::repeat(0u8).take(nframe_bytes as usize);
            mp.frame_buf = Vec::<u8>::from_iter(reps);
            ffmpeg_sys::avpicture_alloc(mp.frame, mp.pix_fmt, mp.target_width as i32, mp.target_height as i32);
            */

            //TODO
            /*
            let _ = ffmpeg_sys::avpicture_fill(mp.frame,
                                               mp.frame_buf.get(0).unwrap(),
                                               mp.pix_fmt,
                                               mp.target_width as i32,
                                               mp.target_height as i32);

            (*mp.frame).format = (*mp.context).pix_fmt as i32;

            // Open the output file.
            static AVIO_FLAG_WRITE: i32 = 2; // XXX: this should be defined by the bindings.
            if ffmpeg_sys::avio_open(&mut (*mp.format_context).pb, path_str.as_ptr(), AVIO_FLAG_WRITE) < 0 {
                panic!("Failed to open the output file.");
            }

            if ffmpeg_sys::avformat_write_header(mp.format_context, ptr::null_mut()) < 0 {
                panic!("Failed to open the output file.");
            }

            if ret < 0 {
                panic!("Could not allocate raw picture buffer");
            }

            */


      }

      return mp;
   }
   pub fn sync_rewarder(&mut self) -> () {
      //let mut sr = self.rewarder.as_mut().unwrap();
      //let &mut (ref mut sender, ref mut receiver) = sr;

      //requires nonblocking before moving into main thread
   }
   pub fn render_frame(&mut self) {
      let width = self.shape.observation_space[0];
      let height = self.shape.observation_space[1];

      /*
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
      */

      self.frame = self.frame + 1;
   }
   pub fn sync_vnc(&mut self) -> () {
      let width = self.shape.observation_space[0];
      let height = self.shape.observation_space[1];

      let pause = 1000 / self.fps;
      std::thread::sleep_ms(pause);
      self.render_frame();

      let mut vnc = self.vnc.as_mut().unwrap();

      //vnc.request_update(vnc::Rect { left: 0, top: 0, width: width as u16, height: height as u16}, false).unwrap();
      for event in vnc.poll_iter() {
         use vnc::client::Event;
         match event {
            Event::PutPixels(vnc_rect, ref pixels) => {
               let mut black_screen = true;
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
                     let x = x as usize;
                     let y = y as usize;
                     self.state.screen[3*(y*width + x)] = pixels[left+2];
                     self.state.screen[3*(y*width + x)+1] = pixels[left+1];
                     self.state.screen[3*(y*width + x)+2] = pixels[left];
                  }
               }}

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
         fps: 20,
         env_id: "gym-core.AirRaid-v0".to_string(),
         max_parallel: 1,
         duration: 180,
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
   pub fn start_remote(&mut self, pi: u32) -> GymRemote {
      self.remote_prep_container(pi);
      let r = GymRemote {
         id: pi,
         fps: self.fps,
         env_id: format!("{: <99}", self.env_id).clone(),
         duration: self.duration,
         record_dst: self.record_dst.clone(),
         mode: "atari".to_string(),
         vnc: None,
         rewarder: None,
         state: GymState {
            screen: vec![0; (ATARI_WIDTH * ATARI_HEIGHT * 3) as usize]
         },
         shape: GymShape {
            action_space: vec![6],
            observation_space: vec![ATARI_WIDTH as usize, ATARI_HEIGHT as usize, 3 as usize],
            reward_max : f64::INFINITY,
            reward_min : f64::NEG_INFINITY
         },
         time: Instant::now(),
         frame: 0
      };
      return r;
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

      self.remote_sanitize();
   }
}

