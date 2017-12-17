// QUESTION/DISCUSSION: linear typing probably most novel interesting
// type system thing for Haskellers

// QUESTION/DISUCSSION: I keep forgetting the ; on the end of lines
// - more generally, this doesn't use indentation based scoping.

use std::fs::File; // QUESTION/DISCUSSION: why is File capitalised but others aren't? Because it's a trait, I guess?
use std::os::unix::io::AsRawFd;
use std::io; // for flushing stdout
use std::io::Read;
use std::io::Write; // for flushing stdout
use std::io::Seek; // QUESTION/DISCUSSION: rustc is good at suggesting traits to import
use std::io::SeekFrom;
// use std::process; // was used for exit, but I'm not exiting...

use std::f64;


// for timestamps:
extern crate time;
use time::Timespec;
use time::Duration;


// QUESTION/DISCUSSION: macros are distinct syntax-wise from
// functions: they always have ! on the end; woo woo pattern matching!
// though this is quite a different language to learn. Basic ideas
// familiar from Haskell though. Some of it has the cursedness of
// regexp "concise" syntax.
// Better syntax integration that a cpp preprocessor.

// frustrates me a bit that the inputs don't look as close to
// function invocations as with cpp macros, but the syntax is
// more flexible.

// specific awkwardnesses: how to take pass multiple arguments
// as per dbg!

// how to take exactly two arguments -eg progress!

macro_rules! dbg {

 ($($x:expr),*)  => {{ 
    print!("debug: ");
    println!($($x),*);
 }};

}

macro_rules! TODO {

 ($($x:expr),*)  => {{ 
    print!("TODO: ");
    println!($($x),*);
 }};

}

// QUESTION/DISCUSSION: what a struggle to get a macro that
// takes two parameters... had a look at assert_eq! source to
// see what they do
macro_rules! progress {
  ( $symbol:expr, $longmsg:expr ) => {
    println!("Progress: ({}) {}", $symbol, $longmsg);
    // print!($symbol);
    io::stdout().flush().ok().expect("flushing stdout for progress log"); 
  };
}





// QUESTION/DISCUSSION:
// it was a bit of a mission to find poll() in libc
// as a lot of discussion was more about how to do common
// things without using poll, but not what I want to do
// with a GPIO pin. Frothy mouthed pragmatism of a lanugage
// with a point - don't I know that from haskell...
extern crate libc;

// QUESTION/DISCUSSION: Haskell style ":" meaning "has type"

// The number of readings per second that will go into the
// shift buffer.
const RPS: usize = 1000; // so that things are sized as ms
const BUFSIZE: usize = RPS * 60;

const BUF_INVALID : u8 = 2;
const TENTHS_INVALID_SENTINEL : i64 = -1;

const GPIO_FILENAME : &'static str = "/sys/class/gpio/gpio25/value";


/* encapsulates the state needed for edge-detecting on a GPIO
   pin */
struct edge_detector {
  file : File   // the file handle for the GPIO pin  
}

fn init_edge_detector(filename : &str) -> edge_detector {
  dbg!("edge_detector: initialising with pin {}", filename);
  let mut fx = File::open(filename).expect("opening GPIO port");
  let mut ed = edge_detector {
    file: fx
  };
  return ed;
}

struct edge {
  level : u8,
  timestamp : Timespec
}

impl Iterator for edge_detector {
  type Item = edge;
  fn next(&mut self) -> Option<edge> {
    dbg!("edge_detector.next starting");

    let fd = self.file.as_raw_fd();

    while {
    dbg!("edge_detector.next start inner loop");
    let mut pfd = libc::pollfd {
        fd: fd,
        events: libc::POLLPRI,
        revents: 0 
      };

    // QUESTION/DISCUSSION: ok so semicolon on the end is significant
    // here: we return the last statement value if it doesn't have
    // a semicolon... if there is a semicolon then there's an implicit
    // void statement? 

    let pollret : i32 = unsafe { libc::poll(&mut pfd, 1, 5000) };

    // even though we time-out here, it is safe to continue with the
    // rest of the processing: we'll treat it as two (or more)
    //  '0's in a row, or two '1's in a row.
    if pollret == 0 {
      progress!("T", "Timeout polling for GPIO pin edge");
    }
    pollret == 0
    } {}
    // QUESTION/DISCUSSION: this is the rust way of writing a do/while 
    // loop: the body goes in the "condition" block. Note no semicolon
    // on the end.


   // QUESTION/DISCUSSION: rustc does a good job warning about
    // unused return value here if expect() is not used to capture
    // it.
    self.file.seek(SeekFrom::Start(0)).expect("seeking to start of GPIO file");

    dbg!("edge_detector.next reading a byte from GPIO");
    let mut gpio_value = [0; 1]; // TODO: what does [0;1] mean?
    self.file.read_exact(&mut gpio_value).expect("reading a byte from GPIO");

    let edge_time = time::get_time();
    // ^ Get this time as close to the read as possible, because
    //   that should be as close to the actual edge as possible.

    dbg!("edge_detector.next read from gpio: value {}", gpio_value[0]);
    dbg!("edge_detector.next, edge time is {} . {}", edge_time.sec, edge_time.nsec);


    // case looks more like haskell pattern matching than C `case`.
    let new_level = match gpio_value[0] {
      48 => 0,
      49 => 1,
      _ => panic!("Unrecognised symbol from GPIO")
    };

    dbg!("edge_detector.next done");
    return Some(edge {
        level: new_level,
        timestamp: edge_time
      });
  }
}

// QUESTION/DISCUSSION: lifetime for edge detector. because we
// mutate it so it needs to be mutable. and in a struct, we have
// to put in a lifetime. There is some lifetime inference going on
// somewhere (see: Lifetime Elision) to mean that an instance of
// edge_detector has the same lifetime as pulse_detector.
struct pulse_detector<'lifetime> {
  ed : &'lifetime mut edge_detector,
  last_edge : edge
  // QUESTION/DISCUSSION: we've got a Maybe type... called Option
}

struct pulse {
  level: u8,
  timestamp: Timespec, // start of pulse
  duration: u8 // in units of 0.1s
}

fn init_pulse_detector(mut e : &mut edge_detector) -> pulse_detector {
  dbg!("pulse_detector: init");
  let next_edge_opt = e.next();
  let next_edge = match next_edge_opt {
    Some(x) => x,
    None => panic!("wasn't expecting edge detector to end")
  };
  return pulse_detector {
    ed: e,
    last_edge: next_edge
  };
}

// QUESTION/DISCUSSION: impl is like "instance" in Haskell.
impl<'a> Iterator for pulse_detector<'a> {

  type Item = pulse; // QUESTION/DISCUSSION: associated type like in Haskell advanced typeclasses
  fn next(&mut self) -> Option<pulse> {
    dbg!("pulse_detector.next"); 

    let new_edge = self.ed.next();
    let next = match new_edge {
      Some(x) => x,
      None => panic!("wasn't expecting edge detector to end")
    };

    let d = next.timestamp - self.last_edge.timestamp;
    let f64_duration = d.num_milliseconds() as f64;

    let rounded_duration = f64::round(f64_duration / 100.0) as u8;

    let p = pulse {
        level: self.last_edge.level,
        timestamp: self.last_edge.timestamp,
        duration: rounded_duration
    };

    dbg!("Pulse level {}, length {} rounded to {}", p.level, d, p.duration);
    self.last_edge = next;
    let p_opt = Some(p);
    return p_opt;
  }

}

fn main() {
  print_banner();

  // this will iterate providing a sequence of timestamped edges.
  let mut edge_detector = init_edge_detector(GPIO_FILENAME);

  let mut pulse_detector = init_pulse_detector(&mut edge_detector);

  loop {
    pulse_detector.next();
  }

/*
  TODO!("get NTP shm");

  dbg!("opening GPIO {}", "foo");


  // QUESTION/DISCUSSION:
  // Vec is a growable vector: we don't need that expandability...
  // so what is a better format? A regular array?
  // Also, what is this "mut" describing? the "buffer" the
  //   reference can change? or the heap content? or both?
  let mut buffer : Vec<u8> = vec![BUF_INVALID; BUFSIZE];
  let mut bufoff : usize = 0;
  let mut oldc : u8 = BUF_INVALID;
  let mut oldtenths : i64 = TENTHS_INVALID_SENTINEL;
  

  dbg!("done initialising");

  loop {

    progress!(".", "poll start");

    let fd = ed.f.as_raw_fd();

    let mut pfd = libc::pollfd {
        fd: fd,
        events: libc::POLLPRI,
        revents: 0 
      };

    // QUESTION/DISCUSSION: ok so semicolon on the end is significant
    // here: we return the last statement value if it doesn't have
    // a semicolon... if there is a semicolon then there's an implicit
    // void statement? 

    let pollret : i32 = unsafe { libc::poll(&mut pfd, 1, 5000) };
    let edge_time = time::get_time();
    // ^ Get this time as close to the read as possible, because
    //   that should be as close to the actual edge as possible.
    dbg!("edge_time: {} . {}", edge_time.sec, edge_time.nsec);

    // even though we time-out here, it is safe to continue with the
    // rest of the processing: we'll treat it as two (or more)
    //  '0's in a row, or two '1's in a row.
    if pollret == 0 {
      progress!("T", "Timeout polling for GPIO pin edge");
    }

  // QUESTION/DISCUSSION: there are too many casts here for my taste.
  // - the C version has some long long casts at the equivalent place
  // but the rust version has more.
    let newtenths = (edge_time.sec as i64) * (RPS as i64) + (edge_time.nsec as i64) / (1000000000 / (RPS as i64));


  // if we've advanced more than one slot since the last edge
    if newtenths != oldtenths && oldtenths != TENTHS_INVALID_SENTINEL {
      let numslots : i64 = newtenths - oldtenths - 1;
      dbg!("debug: Will fill in {} slots", numslots);
      assert!( numslots <= (BUFSIZE as i64) ); // if our poll timeout is more than a minute, we might end up with more than one buffer worth to fill - so need to be sure that the poll timeout is less than a minute. it is 5s at the moment, which is plenty.
// QUESTION/DISCUSSION: this gives a compiler warning because
// 'i' is unused. What is the correct way to do a loop n times?
      for i in 0 .. numslots {
        buffer[bufoff] = oldc;
        bufoff = (bufoff+1) % BUFSIZE;
      }

      print!("{}/{} ", numslots, oldc as char);
      io::stdout().flush().ok().expect("flushing stdout for progress log"); 

      // QUESTION/DISCUSSION: check_decode is called in places that
      // I don't fully understand - twice...
      // for now I'll keep that because that's how I do things in the
      // C version.
      check_decode(&buffer, bufoff);

    }
    oldtenths = newtenths;
 
    // QUESTION/DISCUSSION: should wrap buffer and bufof into
    // a struct (or perhaps something with traits...) 
    check_decode(&buffer, bufoff);

    dbg!("seek in GPIO file to zero");

    // QUESTION/DISCUSSION: rustc does a good job warning about
    // unused return value here if expect() is not used to capture
    // it.
    ed.f.seek(SeekFrom::Start(0)).expect("seeking to start of GPIO file");

    dbg!("reading a byte from GPIO");
    let mut gpio_value = [0; 1]; // TODO: what does [0;1] mean?
    ed.f.read_exact(&mut gpio_value).expect("reading a byte from GPIO");
    dbg!("read from gpio: value {}", gpio_value[0]);

    oldc=gpio_value[0];

  TODO!("MAINLOOP: handle inhibit decode"); 
  }
*/
}

fn print_banner() {
  println!("MSF decoder - Rust version");
  println!("Copyright 2017 Ben Clifford benc@hawaga.org.uk");
  println!("Key:");
  println!("  . = waiting for edge on GPIO input");
  println!("  T = timeout without edge on GPIO input");
  println!("  > = starting decode");
  println!("  X0 = decode failed: insufficient zeroes in scan zone");
  println!("  X1 = decode failed: insufficient ones in scan zone");
  println!("  * = decode inhibition ended");
}

// when check_decode starts, bufoff points at the 
// bit received a minute ago, at the start of the buffer.
fn check_decode(buffer : &Vec<u8>, bufoff : usize) {
  if buffer[bufoff] != ('1' as u8) {
    return;
  }

  TODO!("another shortcut fast return without decoding is implemented in C code, but isn't necessary for actual decoding");

  decode(&buffer, bufoff);
}

// This decodes buffer into 120 bits.
fn decode(buffer : &Vec<u8>, bufoff : usize) {

}
