// QUESTION/DISCUSSION: linear typing probably most novel interesting
// type system thing for Haskellers

// QUESTION/DISUCSSION: I keep forgetting the ; on the end of lines
// - more generally, this doesn't use indentation based scoping.

// QUESTION/DISCUSSION: I use Option as part of Iterator a lot.
// but my streams are infinite - they never end, so that Option
// just gets unwrapped to an impossible error. Probably I could
// replace Iterator with an infinite-stream-like trait.

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

// TODO: commandline optionalness for debug output
macro_rules! dbg {

 ($($x:expr),*)  => {{ 
   //  print!("debug: ");
   //  println!($($x),*);
 }};

}

macro_rules! TODO {

 ($($x:expr),*)  => {{ 
    // print!("TODO: ");
    // println!($($x),*);
 }};

}

// QUESTION/DISCUSSION: what a struggle to get a macro that
// takes two parameters... had a look at assert_eq! source to
// see what they do
macro_rules! progress {
  ( $symbol:expr, $longmsg:expr ) => {
    // println!("Progress: ({}) {}", $symbol, $longmsg);
    print!("{}", $symbol);
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

const BUF_INVALID : u8 = 2;
const TENTHS_INVALID_SENTINEL : i64 = -1;

const GPIO_FILENAME : &'static str = "/sys/class/gpio/gpio12/value";


/* encapsulates the state needed for edge-detecting on a GPIO
   pin */
struct EdgeDetector {
  file : File   // the file handle for the GPIO pin  
}

// QUESTION/DISCUSSION: rust style looks like the init_
// functions should be done as EdgeDetector::new 
// or something like that. A smart-constructor style.
fn init_edge_detector(filename : &str) -> EdgeDetector {
  dbg!("edge_detector: initialising with pin {}", filename);
  let fx = File::open(filename).expect("opening GPIO port");
  let ed = EdgeDetector {
    file: fx
  };
  return ed;
}

struct edge {
  level : u8,
  timestamp : Timespec
}

impl Iterator for EdgeDetector {
  type Item = edge;
  fn next(&mut self) -> Option<edge> {
    progress!(".", "EdgeDetector.next waiting for an edge");

    let fd = self.file.as_raw_fd();

    while {
    dbg!("EdgeDetector.next start inner loop");
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

    let edge_time = time::get_time();
    // ^ Get this time as close to the read as possible, because
    //   that should be as close to the actual edge as possible.

    // QUESTION/DISCUSSION: rustc does a good job warning about
    // unused return value here if expect() is not used to capture
    // it.
    self.file.seek(SeekFrom::Start(0)).expect("seeking to start of GPIO file");

    dbg!("EdgeDetector.next reading a byte from GPIO");
    let mut gpio_value = [0; 1]; // TODO: what does [0;1] mean?
    self.file.read_exact(&mut gpio_value).expect("reading a byte from GPIO");


    dbg!("EdgeDetector.next read from gpio: value {}", gpio_value[0]);
    dbg!("EdgeDetector.next, edge time is {} . {}", edge_time.sec, edge_time.nsec);


    // case looks more like haskell pattern matching than C `case`.
    let new_level = match gpio_value[0] {
      48 => 0,
      49 => 1,
      _ => panic!("Unrecognised symbol from GPIO")
    };

    dbg!("EdgeDetector.next done");
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
// EdgeDetector has the same lifetime as PulseDetector.
struct PulseDetector<'lifetime> {
  ed : &'lifetime mut EdgeDetector,
  last_edge : edge
  // QUESTION/DISCUSSION: we've got a Maybe type... called Option
}

struct pulse {
  level: u8,
  timestamp: Timespec, // start of pulse
  duration: u8 // in units of 0.1s
}

fn init_pulse_detector(mut e : &mut EdgeDetector) -> PulseDetector {
  dbg!("PulseDetector: init");
  let next_edge_opt = e.next();
  let next_edge = match next_edge_opt {
    Some(x) => x,
    None => panic!("wasn't expecting edge detector to end")
  };
  return PulseDetector {
    ed: e,
    last_edge: next_edge
  };
}

// QUESTION/DISCUSSION: impl is like "instance" in Haskell.
impl<'a> Iterator for PulseDetector<'a> {

  type Item = pulse; // QUESTION/DISCUSSION: associated type like in Haskell advanced typeclasses
  fn next(&mut self) -> Option<pulse> {
    dbg!("PulseDetector.next"); 

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
    progress!(p.duration, "logging duration symbol");
    self.last_edge = next;
    let p_opt = Some(p);
    return p_opt;
  }

}


// One-symbol-per-second decoder

// sync to second boundaries
// takes in a pulse iterator
// emits one of 5 states:
//  S for minute sync symbol
//  and 4 other states for a regular 2-bit token
// so perhaps code this as a u8: 0ab and 100
// and represent it in the short output as those numbers
// as a single digit

struct SymbolDecoder<'lifetime> {
  pd : &'lifetime mut PulseDetector<'lifetime>
}


// QUESTION/DISCUSSION: dear god the lifetime annotations. it's
// interesting to see them as type parameters though.
fn init_symbol_decoder<'l>(mut p : &'l mut PulseDetector<'l>) -> SymbolDecoder<'l> {
  dbg!("symbol decoder init");
  
  return SymbolDecoder {
    pd: p
  }
}

impl<'lifetime> Iterator for SymbolDecoder<'lifetime> {
  type Item = u8;
  fn next(&mut self) -> Option<u8> {
    dbg!("SymbolDecoder.next");
    // assume that we're at a long boundary from a previous
    // iteration, or if this is the start, we'll sync as part
    // of what follows.

    // accumulate symbol sequences terminated by a 0-bit >= 5
    // in length.
    // use a 5-element dictionary to turn that into the
    // appropriate output symbol.
    // the maximum sequence length is 4 (1-1-1-7)
    // if we go over that, we've hit a sync error.
    // this pulse is either something to accumulate
    // or a terminator pulse

    // pulse length buffer - with enough capacity to hold the
    // expected sequence length
    let mut pulse_buffer : Vec<u8> = Vec::with_capacity(5);

    let mut cumulative_duration = 0;

    while {
      let next_pulse_opt : Option<pulse> = self.pd.next();

      let next_pulse = match next_pulse_opt {
        None => panic!("next_pulse_opt was None"),
        Some(x) => x
      };

      // accumulate pulse into buffer, with software debounce
      if next_pulse.duration > 0 {
        pulse_buffer.push(next_pulse.duration);
        cumulative_duration += next_pulse.duration;
      }


      // also stop here if the buffer is getting too long 
      // - potential denial-of-service memory exhaustion
      // if we inject only short pulses in. so a bunch of short
      // pulses will generate a bunch of short unrecognised symbols.

      (next_pulse.duration < 5 || next_pulse.level != 0)
        && pulse_buffer.len() < 4
        && cumulative_duration < 10
      }
    {}

    // if it's a terminator pulse, we move onto the
    // symbol lookup stage.
    progress!("/", "looking up symbol");

    dbg!("One-second pulse buffer is {} symbols long", pulse_buffer.len());

    // now an explicit enumeration of the valid possibilities
    // checking only length, not polarity, as there is no ambiguity
    // although it might be a bit more resilient to do so.

    // 5-5  minute marker
    if  pulse_buffer.len() == 2
     && pulse_buffer[0] == 5
     && pulse_buffer[1] == 5 {
      progress!("M", "Minute marker decoded");
      return Some(4); // 4 == MINUTE START SYMBOL
    }  

    // 1-9  double 0
    if  pulse_buffer.len() == 2
     && pulse_buffer[0] == 1 
     && pulse_buffer[1] == 9 {
      progress!("0", "A=0, B=0");
      return Some(0); 
    }  

    // 1-1-1-7  double 01 (a=0, b=1)
    if  pulse_buffer.len() == 4
     && pulse_buffer[0] == 1 
     && pulse_buffer[1] == 1 
     && pulse_buffer[2] == 1 
     && pulse_buffer[3] == 7 {
      progress!("1", "A=0, B=1");
      return Some(1); 
    }  
 
    // 2-8  double 10 (a=1, b=0)
    if  pulse_buffer.len() == 2
     && pulse_buffer[0] == 2 
     && pulse_buffer[1] == 8 {
      progress!("2", "A=1, B=0");
      return Some(2); 
    }  

    // 3-7  double 1
    if  pulse_buffer.len() == 2
     && pulse_buffer[0] == 3 
     && pulse_buffer[1] == 7 {
      progress!("3", "A=1, B=1");
      return Some(3); 
    }  
 
    progress!("x","unrecognised one-second symbol");     
    return Some(8); // 8 == INVALID
  }
}

// TODO: symbol decoder->whole minute bit array iterator
// accumulate a minutes worth of symbols - which should be
// 60 symbols (except at leap second time) - and store them
// in a bit array




// QUESTION/DISCUSSION: this was a mutable reference originally,
// rather than the structure taking ownership of the symbol decoder,
// but see my question on stackoverflow about problems getting it
// to type check the lifetimes:
// https://stackoverflow.com/questions/48158063/reference-does-not-live-long-enough-in-nested-structure

struct MinuteDecoder<'lifetime> {
  sd : SymbolDecoder<'lifetime>
}


fn init_minute_decoder<'l>(mut p : SymbolDecoder<'l>) -> MinuteDecoder<'l> {
  dbg!("minute decoder init");
 
  return MinuteDecoder {
    sd: p
  }
}

impl<'lifetime> Iterator for MinuteDecoder<'lifetime> {
  type Item = [Option<u8>; 60];
  fn next(&mut self) -> Option<[Option<u8>; 60]> {
    dbg!("MinuteDecoder.next");

    // wait for minute marker - which should usually be right
    // away but if we're out of sync, we might need to wait a while

    while {
      progress!("-", "draining until minute marker");
      let c = self.sd.next();
      c != Some(4)
    } {}

    // now read in remaining 59 seconds of data into a buffer
    // TODO: nb not always 59 seconds - in presence of leap seconds
    // To deal with that should probably accumulate symbols
    // in a shift register and use the minute marker to
    // label the *end* not the start of the sequence; or if we want
    // the UT1 offset bits, use the minute marker to label the
    // middle of the sequence.

    // the numbering of syms lines up with the numbering in the NPL
    // protocol document - so we start at 1, the first data carrying
    // symbol.
    let mut syms : [Option<u8>; 60] = [None; 60];
    for offset in 1..60 {
      // QUESTION/DISCUSSION 1..60 constructs the range 1 to 59 (!) not 1 to 60
      syms[offset] = self.sd.next();
      progress!("+", "read symbol in minute decoder");
    }


    progress!("*", "read a minute-sequence in minute decoder");

    // TODO: might want to verify in this decoder, or might
    // have another layer which checks for validity and skips.
    // probably more composable to do it in different layer. 
    return Some(syms);
  }
}

// whole minute bit array -> decoded time
// do the BCD conversions etc. maybe do parity here too

// The structure of a data-carrying symbol. This cannot
// represent the message start symbol or the invalid pseudo-symbol.
// QUESTION/DISCUSSION: Copy,Clone lets us duplicate values of this
// struct so they can be used more like literals - i.e. in the 
// initialisaion of decodedSyms.
/*
#[derive(Copy, Clone)]
struct ABBit {
  a : bool,
  b : bool
} */

// this separates out the A and B bits into separate arrays
// because in the protocol, they're used that way more often.

// QUESTION/DISCUSSION: I'd rather have a numeric bit type here.
// I could use bool but it doesn't get up-casted magically to
// a large numeric type when I try to do maths on it.
// u8 allows that but also allows the data structure to contain
// non-type-checked wrong values. Note difference between the
// datatypes of a,b and of valid, because of their different uses.
// (which is probably a good thing)
struct MinuteData {
  a : [u8; 60],
  b : [u8; 60],
  valid : [bool; 60]
}

struct ABDecoder<'l> {
  md : MinuteDecoder<'l>
}

fn init_ab_decoder<'l>(mut p : MinuteDecoder<'l>) -> ABDecoder<'l> {
  dbg!("AB decoder init");
 
  return ABDecoder {
    md: p
  }
}

impl<'l> Iterator for ABDecoder<'l> {
  type Item = MinuteData;
  
  fn next(&mut self) -> Option<MinuteData> {

    // haskell/functional style would initialise this array as a map result
    // right away rather than iterating over it. TODO
    let mut decoded_syms = MinuteData { a: [0; 60],
                                        b: [0; 60],
                                        valid: [false; 60] };

    'minute_decoder: while {
      let minute_syms_opt = self.md.next();

      let mut valid_so_far = true;

      match minute_syms_opt {
        None => valid_so_far = false,
        Some(minute_syms) => {

          for i in 1..60 { 
            match minute_syms[i] {
              // TODO: could probably decode this bitwise rather than using match
              Some(0) => {
                progress!("p", "Decoded symbol in minute buffer");
                decoded_syms.a[i] = 0;
                decoded_syms.b[i] = 0;
                decoded_syms.valid[i] = true;
              },
              Some(1) => {
                progress!("q", "Decoded symbol in minute buffer");
                decoded_syms.a[i] = 0;
                decoded_syms.b[i] = 1;
                decoded_syms.valid[i] = true;
              },
              Some(2) => {
                progress!("r", "Decoded symbol in minute buffer");
                decoded_syms.a[i] = 1;
                decoded_syms.b[i] = 0;
                decoded_syms.valid[i] = true;

              },
              Some(3) => {
                progress!("s", "Decoded symbol in minute buffer");
                decoded_syms.a[i] = 1;
                decoded_syms.b[i] = 1;
                decoded_syms.valid[i] = true;
              },
              _ => {
                progress!("_", "could not decode a symbol in minute buffer");
                valid_so_far = false;
              }
            }
          }
        }
      }
      false // use !valid_so_far if we want to wait for the whole minute
            // to be valid; otherwise return even if corrupted and rely
            // on later parity and sanity checking.
      // !valid_so_far  
    } {}

    return Some(decoded_syms);
  }

}


struct MessageDecoder<'lifetime> {
  abd : ABDecoder<'lifetime>
}


fn init_message_decoder<'l>(mut p : ABDecoder<'l>) -> MessageDecoder<'l> {
  dbg!("message decoder init");
 
  return MessageDecoder {
    abd: p
  }
}

// decodes a minute-long message into a broken-down unix tm
impl<'l> Iterator for MessageDecoder<'l> {
  type Item = libc::tm;
  
  fn next(&mut self) -> Option<libc::tm> {

   'tryagain: loop {

   let m_opt = self.abd.next();    
   match m_opt { 
   Some(m) => {
    progress!("{", "begin decode minute data");

    // decode one field - eg minutes - just to see if it works

    if !(m.valid[45] && m.valid[46] && m.valid[47] && m.valid[48] && m.valid[49] && m.valid[50] && m.valid [51]) {
      progress!("!", "minutes validity bits: invalid");
      continue 'tryagain;
    } else {
      progress!("M", "minutes validity bits: valid");
    }

    let minute = m.a[45] * 40 + m.a[46] * 20 + m.a[47] * 10 + m.a[48] * 8 + m.a[49] * 4 + m.a[50] * 2 + m.a[51] * 1; 

    if !(m.valid[39] && m.valid[40] && m.valid[41] && m.valid[42] && m.valid[43] && m.valid [44]) {
      progress!("!", "hours validity bits: invalid");
      continue 'tryagain;
    } else {
      progress!("H", "hours validity bits: valid");
    }

    let hour = m.a[39] * 20 + m.a[40] * 10 + m.a[41] * 8 + m.a[42] * 4 + m.a[43] * 2 + m.a[44] * 1; 

    // if we can't decode, loop to try again

    // TODO: hour/minute parity

    if !(m.valid[30] && m.valid[31] && m.valid[32] && m.valid[33] && m.valid[34] && m.valid [35]) {
      progress!("!", "day-of-month validity bits: invalid");
      continue 'tryagain;
    } else {
      progress!("D", "day-of-month validity bits: valid");
    }

    let day_of_month = m.a[30] * 20 + m.a[31] * 10 + m.a[32] * 8 + m.a[33] * 4 + m.a[34] * 2 + m.a[35] * 1; 
   
 

    if !(m.valid[25] && m.valid[26] && m.valid[27] && m.valid[28] && m.valid[29]) {
      progress!("!", "month validity bits: invalid");
      continue 'tryagain;
    } else {
      progress!("M", "month validity bits: valid");
    }

    let month = m.a[25] * 10 + m.a[26] * 8 + m.a[27] * 4 + m.a[28] * 2 + m.a[29] * 1; 

    if !(m.valid[17] && m.valid[18] && m.valid[19] && m.valid[19] && m.valid[20] && m.valid[21] && m.valid[22] && m.valid[23] && m.valid[24]) {
      progress!("!", "year validity bits: invalid");
      continue 'tryagain;
    } else {
      progress!("Y", "year validity bits: valid");
    }

    let year = m.a[17] * 80 + m.a[18] * 40 + m.a[19] * 20 + m.a[20] * 10 + m.a[21] * 8 + m.a[22] * 4 + m.a[23] * 2 + m.a[24] * 1; 

    progress!("}", "end decode minute data");

    println!("");
    println!("Decode: {}-{}-{} {}:{}", year, month, day_of_month, hour, minute);

    let mut clocktime = libc::tm {
        tm_sec: 0,
        tm_min: minute as i32,
        tm_hour: hour as i32,
        tm_mday: day_of_month as i32,

// QUESTION/DISCUSSION:  phrasing this as `(month -1) as i32` means that
// if we get a corrupt month value 0, rust dies with an error about the
// u8 underflowing on subtraction. Getting that corrupt 0 into being an
// i32 first means that we've got the range we need. It's nice that
// rust detects the underflow.
        tm_mon: (month as i32) - 1,
        tm_year: (year as i32) + 100,
        tm_isdst: 0,
        tm_gmtoff: 0, // unused
        tm_wday: 0, // unused - even though it's in broadcast
        tm_yday: 0, // unused
        tm_zone: std::ptr::null::<u8>()  // unused
      };


    return Some(clocktime);
   },
   None => panic!("MessageDecoder returned None")
  }
  }
  }
}


fn main() {
  print_banner();

  let ntpmem = getShmTime(3);

  // this will iterate providing a sequence of timestamped edges.
  let mut edge_detector = init_edge_detector(GPIO_FILENAME);

  let mut pulse_detector = init_pulse_detector(&mut edge_detector);

  let mut symbol_decoder = init_symbol_decoder(&mut pulse_detector);

  let mut minute_decoder = init_minute_decoder(symbol_decoder);

  let mut ab_decoder = init_ab_decoder(minute_decoder);

  let mut message_decoder = init_message_decoder(ab_decoder);

  loop {
    println!(""); // get a new line so that each major loop iteration has a line break
    progress!(">", "start of main infinite loop iteration");

    // get the time
    let clocktime_opt = message_decoder.next();
    match clocktime_opt {
     None => panic!("message decoder returned None"),
     Some(mut clocktime) => {

    // now feed it to NTP via SHM.

    let tt = unsafe { libc::mktime(&mut clocktime) };

    let mut tv = libc::timeval { tv_sec: 0, tv_usec: 0};

    unsafe { libc::gettimeofday(&mut tv, std::ptr::null_mut::<libc::c_void>()) };

    unsafe {

      assert!(!ntpmem.is_null());

      // TODO minor bug
      // this assumes that NTP will have marked the memory as invalid
      // already, from a previous read: otherwise we should hold off the
      // write as we may be corrupting memory as it is being read by
      // ntpd.
      std::ptr::write_volatile(&mut (*ntpmem).clockTimeStampSec, tt);
      std::ptr::write_volatile(&mut (*ntpmem).clockTimeStampUSec, 0);

      std::ptr::write_volatile(&mut (*ntpmem).receiveTimeStampSec, tv.tv_sec);
      std::ptr::write_volatile(&mut (*ntpmem).receiveTimeStampUSec, tv.tv_usec);

      std::ptr::write_volatile(&mut (*ntpmem).valid, 1);
    }
    progress!("N", "wrote time to ntpd");

  /* 
    ntpmem->clockTimeStampSec = clocksec;
    ntpmem->clockTimeStampUSec = 0;

    gettimeofday(tv, tz);
    ntpmem->receiveTimeStampSec = tv->tv_sec;
    ntpmem->receiveTimeStampUSec = tv->tv_usec;
    ntpmem->valid = 1;
*/
 

  }
  }
  }







// SHM stuff:

// this is a constant from ntpd.
const NTPD_BASE : libc::key_t = 0x4e545030;


// this is a struct definition from ntpd,
// transcribed into rust:

struct ShmTime {
  mode : libc::c_int,
  count : libc::c_int,
  clockTimeStampSec : libc::time_t,
  clockTimeStampUSec : libc::c_int,
  receiveTimeStampSec : libc::time_t,
  receiveTimeStampUSec : libc::c_int,
  leap : libc::c_int,
  precision : libc::c_int,
  nsamples : libc::c_int,
  valid : libc::c_int,
  clockTimeStampNSec : libc::c_uint,
  receiveTimeStampNSec : libc::c_uint,
  dummy : [libc::c_int; 8]
}

/*
struct shmTime {
        int    mode; /* 0 - if valid is set:
                      *       use values,
                      *       clear valid
                      * 1 - if valid is set:
                      *       if count before and after read of data is equal:
                      *         use values
                      *       clear valid
                      */
        volatile int    count;
        time_t          clockTimeStampSec;
        int             clockTimeStampUSec;
        time_t          receiveTimeStampSec;
        int             receiveTimeStampUSec;
        int             leap;
        int             precision;
        int             nsamples;
        volatile int    valid;
        unsigned        clockTimeStampNSec;     /* Unsigned ns timestamps */
        unsigned        receiveTimeStampNSec;   /* Unsigned ns timestamps */
        int             dummy[8];
};
*/

// following function based on BSD-licensed code from gpsd
// TODO: this attaches the shared memory into process memory
// space, which looks a bit like an allocation. It would be
// nice (rust-like), I think, if the rust scoping mechanism
// caused it to be unattached automatically when a suitable
// object goes out of scope.

fn getShmTime(unit: libc::c_int) -> *mut ShmTime {
  println!("Getting ntpd shm unit {}", unit);

  // QUESTION/DISCUSSION: use ::<> to pass in type parameters to sizeof.
  // What does that "type parameter" passing look like in haskell? I know what it looks liek in Idris - it's just a parameter.

  let key = NTPD_BASE + unit;

  println!("key is {}", key);
  println!("size is {}", std::mem::size_of::<ShmTime>());

  let shmid = unsafe { libc::shmget(key, std::mem::size_of::<ShmTime>(), libc::IPC_CREAT | 0o666) };

  // QUESTION/DISCUSSION: back to the land of C style error handling...
  if shmid == -1 {
    println!("errno = {}", std::io::Error::last_os_error());
    panic!("Could not get shmid for NTP communication");
  }

  let shmptr = unsafe { libc::shmat(shmid, std::ptr::null::<libc::c_void>(), 0) };

  if (shmptr as libc::c_int) == -1 {
    panic!("Could not attach shared memory for NTP communication");
  }

  // QUESTION/DISCUSSION: apparently can cast a void pointer
  // to an ShmTime pointer without declaring this unsafe.
  return shmptr as *mut ShmTime;
}

/*
// following function based on BSD-licensed code from gpsd
static volatile struct shmTime *getShmTime(int unit)
{
#define NTPD_BASE 0x4e545030
    int shmid;
    unsigned int perms;
    volatile struct shmTime *p;
    // set the SHM perms the way ntpd does
    if (unit < 2) {
        // we are root, be careful
        perms = 0600;
    } else {
        // we are not root, try to work anyway
        perms = 0666;
    }

    shmid = shmget((key_t) (NTPD_BASE + unit),
                   sizeof(struct shmTime), (int)(IPC_CREAT | perms));
    if (shmid == -1) {
        printf( "NTPD shmget(%ld, %zd, %o) fail: %s\n",
                    (long int)(NTPD_BASE + unit), sizeof(struct shmTime),
                    (int)perms, strerror(errno));
        return NULL;
    } 
    p = (struct shmTime *)shmat(shmid, 0, 0);
    /*@ -mustfreefresh */
    if ((int)(long)p == -1) {
        printf("NTPD shmat failed: %s\n",
                    strerror(errno));
        return NULL;
    }
    return p;
    /*@ +mustfreefresh */
}
*/









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
  println!("Copyright 2017-2018 Ben Clifford benc@hawaga.org.uk");
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
