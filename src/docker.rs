use std::process::Command;
use std::{thread, time, str};
use std::string::String;
use std::io::prelude::*;
use std::io::stdin;
use std::net::TcpStream;
use std::sync::mpsc::channel;

extern crate hyper;
use self::hyper::header::{Headers, Authorization, Basic};

extern crate websocket;
use self::websocket::{Message, Sender, Receiver};
use self::websocket::client::request::Url;
use self::websocket::client::Request;
use self::websocket::Client;
use self::websocket::header::extensions::Extension;

extern crate mpeg_encoder;

fn write_to_mpeg() {
   let mut e = mpeg_encoder::Encoder::new("a.mpeg", 1, 1);
   e.encode_rgb(1, 1, &[0], false);
}

pub fn spawn_env() {
   let s = Command::new("docker")
            .arg("run")
            .arg("-p").arg("5900:5900")
            .arg("-p").arg("15900:15900")
            .arg("quay.io/openai/universe.gym-core:0.20.0")
            .spawn();
   print!("{}", "spawned docker process at 5900 / 15900");
   thread::sleep(time::Duration::from_millis(5000));

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

   
   let reset_cmd = "{\"method\":\"v0.env.reset\",\"body\":{\"seed\":1,\"env_id\":\"gym-core.Amidar-v0\",\"fps\":10},\"headers\":{\"sent_at\":0,\"episode_id\":0,\"message_id\":0}}";
   let message = Message::text(String::from(reset_cmd));
   match tx.send(message) {
      Ok(()) => (),
      Err(e) => {
         println!("Main Loop: {:?}", e);
      }
   }

   write_to_mpeg();

   // We're exiting
   println!("Waiting for child threads to exit");
   let _ = send_loop.join();
   let _ = receive_loop.join();
   println!("Exited");

}
