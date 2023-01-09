//  PROFILING.rs
//    by Lut99
// 
//  Created:
//    06 Jan 2023, 11:47:00
//  Last edited:
//    09 Jan 2023, 18:50:45
//  Auto updated?
//    Yes
// 
//  Description:
//!   Contains some structs that we use to carry around profiling
//!   information.
// 

use std::convert::TryFrom;
use std::fmt::{Debug, Display, Formatter, Result as FResult};
use std::time::{Duration, SystemTime};

use enum_debug::EnumDebug;
use num_traits::AsPrimitive;
use prost::{Message, Oneof};
use prost_types::Timestamp;
use serde::{Deserialize, Serialize};
use serde::de::{self, Deserializer, SeqAccess, Visitor};
use serde::ser::{Serializer, SerializeTuple};


// /***** HELPER MACROS *****/
// /// A helper macro for immediately showing the timing from a string.
// #[macro_export]
// macro_rules! timing {
//     ($raw:expr, $format_fn:ident) => {
//         serde_json::from_str::<specifications::profiling::Timing>($raw).map(|t| format!("{}", t.$format_fn())).unwrap_or("<unparseable>".into())
//     };
// }





/***** AUXILLARY *****/
/// Defines a formatter for the timing that writes it in milliseconds, microseconds or nanoseconds.
/// 
/// It tries to write the biggest of those _unless_ it is zero.
#[derive(Debug)]
pub struct TimingFormatter<'a> {
    /// The Timing to format.
    timing : &'a Timing,
}
impl<'a> Display for TimingFormatter<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        // Try milliseconds first
        let time: u128 = self.timing.elapsed_ms();
        if time > 0 {
            return write!(f, "{}ms", time);
        }

        // Microseconds next
        let time: u128 = self.timing.elapsed_us();
        if time > 0 {
            return write!(f, "{}us", time);
        }

        // Nanoseconds as last
        write!(f, "{}ns", self.timing.elapsed_ns())
    }
}



/// Defines a helper type that automatically calls `Timing::start()` when created and `Timing::stop()` when destroyed.
#[derive(Debug)]
pub struct TimingGuard<'t>(&'t mut Timing);
impl<'t> TimingGuard<'t> {
    /// Creates a new TimingGuard based on the given Timing.
    /// 
    /// # Returns
    /// A new TimingGuard instance that has already activated the given timer. When it goes out-of-scope, will automatically stop it.
    #[inline]
    pub fn new(timing: &'t mut Timing) -> Self {
        timing.start();
        Self(timing)
    }
}
impl<'t> Drop for TimingGuard<'t> {
    #[inline]
    fn drop(&mut self) {
        self.0.stop();
    }
}



/// Defines a start/stop pair as far as profiling goes.
/// 
/// # A note on serialization
/// Unfortunately, it is impossible to serialize / deserialize an Instant, on which the Timing relies. Instead, when you serialize it, you will only serialize the elapsed time. Deserializing a Timing will thus give you a Timing that has different Instants, but leads to the same results when calling `Timing::elapsed_XX()`.
#[derive(Clone, Eq, Hash, Message, PartialEq)]
pub struct Timing {
    /// The start moment of the timing
    #[prost(tag = "1", optional, message)]
    start : Option<Timestamp>,
    /// The stop moment of the timing
    #[prost(tag = "2", optional, message)]
    stop  : Option<Timestamp>,
}
impl Timing {
    /// Constructor for the Timing that initializes it as empty.
    /// 
    /// # Returns
    /// A new Timing instance on which you have to call `Timing::start()` and `Timing::stop()` still.
    #[inline]
    pub fn new() -> Self { Self{ start: None, stop: None } }

    /// Constructor for the Timing that immediately starts timing.
    /// 
    /// # Returns
    /// A new Timing instance with the start time set to now. You still have to call `Timing::stop`.
    #[inline]
    pub fn new_start() -> Self { Self{ start: Some(SystemTime::now().into()), stop: None } }

    /// Constructor for the Timing that returns it such that it records no time elapsed.
    /// 
    /// This is achieved by setting both the internal start and stop time to the same (current) time.
    /// 
    /// # Returns
    /// A new Timing instance that is guaranteed to return 0 on any `Timing::elapsed_XX()` call.
    #[inline]
    pub fn none() -> Self {
        let time: SystemTime = SystemTime::now();
        Self {
            start : Some(time.into()),
            stop  : Some(time.into()),
        }
    }



    /// Starts the timing.
    /// 
    /// If it has been started already, simply overrides the start time with the current time.
    /// 
    /// Always resets the stop time to be unset.
    #[inline]
    pub fn start(&mut self) {
        self.start = Some(SystemTime::now().into());
        self.stop  = None;
    }

    /// Stops the timing.
    /// 
    /// If it has been stopped already, simply overrides the stop time with the current time.
    /// 
    /// # Panics
    /// This function will panic if `Timing::start()` has not yet been called.
    #[inline]
    pub fn stop(&mut self) {
        if self.start.is_none() { panic!("Cannot call `Timing::stop()` without calling `Timing::start()` first"); }
        self.stop = Some(SystemTime::now().into());
    }
    /// A weird but convenient function that takes ownership of `self`, stops its timing, and then casts it to whatever we want to return.
    /// 
    /// # Returns
    /// The same `self` as past in for easy timing and conversion.
    /// 
    /// # Panics
    /// This function will panic if `Timing::start()` has not yet been called.
    #[inline]
    pub fn into_stop<T>(mut self) -> T
    where
        Self: Into<T>,
    {
        if self.start.is_none() { panic!("Cannot call `Timing::stop()` without calling `Timing::start()` first"); }
        self.stop = Some(SystemTime::now().into());
        self.into()
    }

    /// Returns a TimingGuard which will call `Timing::start()` when created and `Timing::stop()` when it is destroyed (i.e., goes out-of-scope).
    #[inline]
    pub fn guard(&mut self) -> TimingGuard { TimingGuard::new(self) }



    /// Formats the Timing neatly into milliseconds, microseconds or nanoseconds (whichever one is the most appropriate).
    /// 
    /// # Returns
    /// A TimingFormatter struct that does the formatting work and implements Display.
    /// 
    /// # Panics(ish)
    /// `Display`ing the returned TimingFormatter may panic if either `Timing::start()` of `Timing::stop` has not been called.
    #[inline]
    pub fn display(&self) -> TimingFormatter { TimingFormatter{ timing: self } }

    /// Returns whether this Timing has been successfully started and stopped (i.e., a time taken can be computed).
    #[inline]
    pub fn is_taken(&self) -> bool { self.start.is_some() && self.stop.is_some() }

    /// Returns the time taken in milliseconds.
    /// 
    /// # Panics
    /// This function will panic if the timing is not successfully taken (i.e., either `Timing::start()` of `Timing::stop` has not been called).
    #[inline]
    pub fn elapsed_ms(&self) -> u128 {
        if let (Some(start), Some(stop)) = (self.start.clone(), self.stop.clone()) {
            // Attempt to parse the timestamps
            SystemTime::try_from(stop).unwrap().duration_since(SystemTime::try_from(start).unwrap()).unwrap_or(Duration::ZERO).as_millis()
        } else {
            panic!("Cannot call `Timing::elapsed_ms()` without first calling both `Timing::start()` and `Timing::stop()`");
        }
    }
    /// Returns the time taken in microseconds.
    /// 
    /// # Panics
    /// This function will panic if the timing is not successfully taken (i.e., either `Timing::start()` of `Timing::stop` has not been called).
    #[inline]
    pub fn elapsed_us(&self) -> u128 {
        if let (Some(start), Some(stop)) = (self.start.clone(), self.stop.clone()) {
            SystemTime::try_from(stop).unwrap().duration_since(SystemTime::try_from(start).unwrap()).unwrap_or(Duration::ZERO).as_micros()
        } else {
            panic!("Cannot call `Timing::elapsed_us()` without first calling both `Timing::start()` and `Timing::stop()`");
        }
    }
    /// Returns the time taken in nanoseconds.
    /// 
    /// # Panics
    /// This function will panic if the timing is not successfully taken (i.e., either `Timing::start()` of `Timing::stop` has not been called).
    #[inline]
    pub fn elapsed_ns(&self) -> u128 {
        if let (Some(start), Some(stop)) = (self.start.clone(), self.stop.clone()) {
            SystemTime::try_from(stop).unwrap().duration_since(SystemTime::try_from(start).unwrap()).unwrap_or(Duration::ZERO).as_nanos()
        } else {
            panic!("Cannot call `Timing::elapsed_ns()` without first calling both `Timing::start()` and `Timing::stop()`");
        }
    }
}
impl AsRef<Timing> for Timing {
    #[inline]
    fn as_ref(&self) -> &Self { self }
}
impl Serialize for Timing {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let (Some(start), Some(stop)) = (&self.start, &self.stop) {
            // Simply write the values
            let mut tuple: S::SerializeTuple = serializer.serialize_tuple(4)?;
            tuple.serialize_element(&start.seconds)?;
            tuple.serialize_element(&start.nanos)?;
            tuple.serialize_element(&stop.seconds)?;
            tuple.serialize_element(&stop.nanos)?;
            tuple.end()
        } else {
            panic!("Cannot serialize a Timing that is not yet taken (call `Timing::start()` and `Timing::stop()` first)");
        }
    }
}
impl<'de> Deserialize<'de> for Timing {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        /// A visitor for the Timing
        struct TimingVisitor;
        impl<'de> Visitor<'de> for TimingVisitor {
            type Value = Timing;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "a timing")
            }

            fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut seq = seq;

                // The start
                let secs  : i64 = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let nanos : i32 = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(1, &self))?;
                let start : Timestamp = Timestamp { seconds: secs, nanos };

                // The stop
                let secs  : i64 = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(2, &self))?;
                let nanos : i32 = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(3, &self))?;
                let stop  : Timestamp = Timestamp { seconds: secs, nanos };

                // That's enough to re-create ourselves
                Ok(Timing {
                    start : Some(start),
                    stop  : Some(stop),
                })
            }
        }

        // Simply visit the timing
        deserializer.deserialize_seq(TimingVisitor)
    }
}





/***** LIBRARY *****/
/// Defines some useful trait for unifying access to profiles.
pub trait Profile<'de>: Clone + Debug + Deserialize<'de> + Message + Serialize {}



/// Defines the profile times we're interested in as far as the instance is concerned.
#[derive(Clone, Deserialize, Message, Serialize)]
pub struct DriverProfile {
    /// Defines the timing for the entire snippet, including everything.
    #[prost(tag = "1", required, message)]
    pub snippet : Timing,

    /// Defines the timing for the non-async part of the driver.
    #[prost(tag = "2", required, message)]
    pub request_overhead   : Timing,
    /// Defines the timing for the async part of the driver.
    #[prost(tag = "3", required, message)]
    pub request_processing : Timing,
    /// Defines the timing for parsing a workflow.
    #[prost(tag = "4", required, message)]
    pub workflow_parse     : Timing,

    /// Defines the timing for executing a workflow.
    #[prost(tag = "5", required, message)]
    pub execution         : Timing,
    /// Defines the timings of the VM itself.
    #[prost(tag = "6", required, message)]
    pub execution_details : VmProfile,
}
impl DriverProfile {
    /// Constructor for the InstanceProfile that initializes it with all timings uninitialized.
    /// 
    /// # Returns
    /// A new InstanceProfile instance with all the internal timings uninitialized.
    #[inline]
    pub fn new() -> Self {
        Self {
            snippet : Timing::new(),

            request_overhead   : Timing::new(),
            request_processing : Timing::new(),
            workflow_parse     : Timing::new(),

            execution         : Timing::new(),
            execution_details : VmProfile::new(),
        }
    }
}
impl AsRef<DriverProfile> for DriverProfile {
    #[inline]
    fn as_ref(&self) -> &Self { self }
}
impl<'de> Profile<'de> for DriverProfile {}



/// Defines the profile times we're interested in as far as the VM is concerned.
#[derive(Clone, Deserialize, Message, Serialize)]
pub struct VmProfile {
    /// Defines the timing for the entire snippet, including everything.
    #[prost(tag = "1", required, message)]
    pub snippet : Timing,

    /// The time it takes to plan the workflow from the VM's perspective.
    #[prost(tag = "2", required, message)]
    pub planning         : Timing,
    /// The time it takes to plan the workflow from the planner's perspective.
    #[prost(tag = "3", required, message)]
    pub planning_details : PlannerProfile,

    /// The time it takes to run the plan.
    #[prost(tag = "4", required, message)]
    pub running         : Timing,
    /// The details about running.
    #[prost(tag = "5", required, message)]
    pub running_details : ThreadProfile,
}
impl VmProfile {
    /// Constructor for the VmProfile.
    /// 
    /// # Returns
    /// A new VmProfile with all of its timings uninitialized.
    #[inline]
    pub fn new() -> Self {
        Self {
            snippet : Timing::new(),

            planning         : Timing::new(),
            planning_details : PlannerProfile::new(),

            running         : Timing::new(),
            running_details : ThreadProfile::new(),
        }
    }
}
impl AsRef<VmProfile> for VmProfile {
    #[inline]
    fn as_ref(&self) -> &Self { self }
}
impl<'de> Profile<'de> for VmProfile {}



/// Defines the profile for a single function.
#[derive(Clone, Deserialize, Message, Serialize)]
pub struct PlannerFunctionProfile {
    /// The name of the function.
    #[prost(tag = "1", required, string)]
    pub name   : String,
    /// The timing of the function.
    #[prost(tag = "2", required, message)]
    pub timing : Timing,
}

/// Defines the profile times we're interested in as far as the planner is concerned.
#[derive(Clone, Deserialize, Message, Serialize)]
pub struct PlannerProfile {
    /// The time it takes to plan an entire snippet.
    #[prost(tag = "1", required, message)]
    pub snippet : Timing,

    /// The overhead of receiving the request.
    #[prost(tag = "2", required, message)]
    pub request_overhead     : Timing,
    /// The overhead of parsing the workflow.
    #[prost(tag = "3", required, message)]
    pub workflow_parse       : Timing,
    /// The overhead of getting other information.
    #[prost(tag = "4", required, message)]
    pub information_overhead : Timing,

    /// The time it takes for the actual planning algorithm.
    #[prost(tag = "5", required, message)]
    pub planning       : Timing,
    /// The time it takes to plan the main function with everything
    #[prost(tag = "6", required, message)]
    pub main_planning  : Timing,
    /// The time it takes to plan *all* functions
    #[prost(tag = "7", required, message)]
    pub funcs_planning : Timing,
    /// The time it takes to plan the edges in each of the functions (main included).
    #[prost(tag = "8", repeated, message)]
    pub func_planning  : Vec<PlannerFunctionProfile>,
}
impl PlannerProfile {
    /// Constructor for the PlannerProfile that intializes all timings to be unset.
    /// 
    /// # Returns
    /// A new PlannerProfile instance.
    #[inline]
    pub fn new() -> Self {
        Self {
            snippet : Timing::new(),

            request_overhead     : Timing::new(),
            workflow_parse       : Timing::new(),
            information_overhead : Timing::new(),

            planning       : Timing::new(),
            main_planning  : Timing::new(),
            funcs_planning : Timing::new(),
            func_planning  : Vec::new(),
        }
    }



    /// Returns a guard for a new function timing.
    /// 
    /// # Arguments
    /// - `name`: The name of the function we are planning.
    /// 
    /// # Returns
    /// A new TimeGuard instance that, when dropped, will complete the timing for planning a specific function.
    pub fn guard_func(&mut self, name: impl Into<String>) -> TimingGuard {
        let name: String = name.into();

        // Insert a new one and then return it
        self.func_planning.push(PlannerFunctionProfile{ name, timing: Timing::new() });
        let last_elem: usize = self.func_planning.len() - 1;
        self.func_planning[last_elem].timing.guard()
    }
}
impl AsRef<PlannerProfile> for PlannerProfile {
    #[inline]
    fn as_ref(&self) -> &Self { self }
}
impl<'de> Profile<'de> for PlannerProfile {}



/// Profiles timings for the thread-part of a VM (i.e., the most hardcore instruction executor).
#[derive(Clone, Deserialize, Message, Serialize)]
pub struct ThreadProfile {
    /// The execution time of the entire snippet.
    #[prost(tag = "1", required, message)]
    pub snippet : Timing,

    /// The timings of each of the edges.
    #[prost(tag = "2", repeated, message)]
    pub edges : Vec<EdgeProfile>,
}
impl ThreadProfile {
    /// Constructor for the ThreadProfile that intializes all timings to be unset.
    /// 
    /// # Returns
    /// A new ThreadProfile instance.
    #[inline]
    pub fn new() -> Self {
        Self {
            snippet : Timing::new(),

            edges : Vec::new(),
        }
    }
}
impl AsRef<ThreadProfile> for ThreadProfile {
    #[inline]
    fn as_ref(&self) -> &Self { self }
}
impl<'de> Profile<'de> for ThreadProfile {}

// /// Profiles timings for each edge.
// #[derive(Clone, Deserialize, Message, Serialize)]
// pub struct EdgeProfile {
//     /// The function that we're executing in.
//     #[prost(tag = "1", required, uint64)]
//     pub func   : u64,
//     /// The edge we're executing in that function.
//     #[prost(tag = "2", required, uint64)]
//     pub edge   : u64,
//     /// The time it taken this edge in its entirety.
//     #[prost(tag = "3", required, message)]
//     pub timing : Timing,
// }
// impl EdgeProfile {
//     /// Constructor for the EdgeProfile that intializes all timings to be unset.
//     /// 
//     /// # Arguments
//     /// - `func`: The index of the function we are executing in.
//     /// - `edge`: The index of the edge we are executing in that function.
//     /// 
//     /// # Returns
//     /// A new EdgeProfile instance.
//     #[inline]
//     pub fn new(func: usize, edge: usize) -> Self {
//         Self {
//             func   : func as u64,
//             edge   : edge as u64,
//             timing : Timing::new(),
//         }
//     }
// }
// impl AsRef<EdgeProfile> for EdgeProfile {
//     #[inline]
//     fn as_ref(&self) -> &Self { self }
// }



/// Provides timings for an individual edge.
#[derive(Clone, Deserialize, Message, Serialize)]
pub struct EdgeProfile {
    /// The index of the function we executed.
    #[prost(tag = "1", required, uint64)]
    pub func : u64,
    /// The index of the edge we executed within that function.
    #[prost(tag = "2", required, uint64)]
    pub edge : u64,

    /// The timings for this edge.
    #[prost(tags = "3,4,5,6", oneof = "EdgeTimings")]
    pub timings : Option<EdgeTimings>,
}
impl EdgeProfile {
    /// Constructor for the EdgeProfile that initializes it with the given function & edge indices.
    /// 
    /// # Arguments
    /// - `func`: The index of the function we executed.
    /// - `edge`: The index of the edge we executed within the given `func`tion.
    /// 
    /// # Returns
    /// A new EdgeProfile that has the given function and edge indices, but unitialized timings.
    #[inline]
    pub fn new(func: impl AsPrimitive<u64>, edge: impl AsPrimitive<u64>) -> Self {
        Self {
            func : func.as_(),
            edge : edge.as_(),

            timings : None,
        }
    }

    /// Constructor for the EdgeProfile that initializes it with the given function & edge indices and the given timings.
    /// 
    /// # Arguments
    /// - `func`: The index of the function we executed.
    /// - `edge`: The index of the edge we executed within the given `func`tion.
    /// - `timings`: The EdgeTimings struct to initialize ourselves with.
    /// 
    /// # Returns
    /// A new EdgeProfile that's already ready to go.
    #[inline]
    pub fn with_timings(func: impl AsPrimitive<u64>, edge: impl AsPrimitive<u64>, timings: EdgeTimings) -> Self {
        Self {
            func : func.as_(),
            edge : edge.as_(),

            timings : Some(timings),
        }
    }
}

/// Contains the actual timings of an edge. What might be found here differs on the type of Edge referenced.
#[derive(Clone, Deserialize, EnumDebug, Oneof, Serialize)]
#[enum_debug(name)]
pub enum EdgeTimings {
    /// Contains profiles for a Node.
    #[prost(tag = "3", message)]
    Node(NodeProfile),
    /// Contains the profiles of the linear edge itself, plus the individual timings of the processed instructions.
    #[prost(tag = "4", message)]
    Linear(LinearProfile),
    /// Contains profiles of the various branches of a parallel statement. Note, though, that we note this under the join because that's the only point we know it, of course.
    #[prost(tag = "5", message)]
    Join(JoinProfile),
    /// Contains timing profiles for the call instruction itself + any time spent executing a builtin.
    #[prost(tag = "6", message)]
    Call(CallProfile),

    /// Contains profiles for a very simple edge that is not one of the already defined edges.
    #[prost(tag = "7", message)]
    Other(Timing),
}
impl EdgeTimings {
    /// Returns the internal NodeProfile as if this was an `EdgeTimings::Node`.
    /// 
    /// # Returns
    /// A reference to the internal NodeProfile.
    /// 
    /// # Panics
    /// This function panics if this as not, in fact, an `EdgeTimings::Node`.
    #[inline]
    pub fn node(&self) -> &NodeProfile {
        if let Self::Node(prof) = self {
            prof
        } else {
            panic!("Cannot unwrap EdgeTimings::{} as EdgeTimings::Node", self.variant());
        }
    }
    /// Returns the internal NodeProfile mutably as if this was an `EdgeTimings::Node`.
    /// 
    /// # Returns
    /// A mutable reference to the internal NodeProfile.
    /// 
    /// # Panics
    /// This function panics if this as not, in fact, an `EdgeTimings::Node`.
    #[inline]
    pub fn node_mut(&mut self) -> &mut NodeProfile {
        if let Self::Node(prof) = self {
            prof
        } else {
            panic!("Cannot unwrap EdgeTimings::{} as EdgeTimings::Node", self.variant());
        }
    }

    /// Returns the internal LinearProfile as if this was an `EdgeTimings::Linear`.
    /// 
    /// # Returns
    /// A reference to the internal LinearProfile.
    /// 
    /// # Panics
    /// This function panics if this as not, in fact, an `EdgeTimings::Join`.
    #[inline]
    pub fn linear(&self) -> &LinearProfile {
        if let Self::Linear(prof) = self {
            prof
        } else {
            panic!("Cannot unwrap EdgeTimings::{} as EdgeTimings::Linear", self.variant());
        }
    }
    /// Returns the internal LinearProfile mutably as if this was an `EdgeTimings::Linear`.
    /// 
    /// # Returns
    /// A mutable reference to the internal LinearProfile.
    /// 
    /// # Panics
    /// This function panics if this as not, in fact, an `EdgeTimings::Linear`.
    #[inline]
    pub fn linear_mut(&mut self) -> &mut LinearProfile {
        if let Self::Linear(prof) = self {
            prof
        } else {
            panic!("Cannot unwrap EdgeTimings::{} as EdgeTimings::Linear", self.variant());
        }
    }

    /// Returns the internal JoinProfile as if this was an `EdgeTimings::Join`.
    /// 
    /// # Returns
    /// A reference to the internal JoinProfile.
    /// 
    /// # Panics
    /// This function panics if this as not, in fact, an `EdgeTimings::Join`.
    #[inline]
    pub fn join(&self) -> &JoinProfile {
        if let Self::Join(prof) = self {
            prof
        } else {
            panic!("Cannot unwrap EdgeTimings::{} as EdgeTimings::Join", self.variant());
        }
    }
    /// Returns the internal JoinProfile mutably as if this was an `EdgeTimings::Join`.
    /// 
    /// # Returns
    /// A mutable reference to the internal JoinProfile.
    /// 
    /// # Panics
    /// This function panics if this as not, in fact, an `EdgeTimings::Join`.
    #[inline]
    pub fn join_mut(&mut self) -> &mut JoinProfile {
        if let Self::Join(prof) = self {
            prof
        } else {
            panic!("Cannot unwrap EdgeTimings::{} as EdgeTimings::Join", self.variant());
        }
    }

    /// Returns the internal CallProfile as if this was an `EdgeTimings::Call`.
    /// 
    /// # Returns
    /// A reference to the internal CallProfile.
    /// 
    /// # Panics
    /// This function panics if this as not, in fact, an `EdgeTimings::Call`.
    #[inline]
    pub fn call(&self) -> &CallProfile {
        if let Self::Call(prof) = self {
            prof
        } else {
            panic!("Cannot unwrap EdgeTimings::{} as EdgeTimings::Call", self.variant());
        }
    }
    /// Returns the internal CallProfile mutably as if this was an `EdgeTimings::Call`.
    /// 
    /// # Returns
    /// A mutable reference to the internal CallProfile.
    /// 
    /// # Panics
    /// This function panics if this as not, in fact, an `EdgeTimings::Call`.
    #[inline]
    pub fn call_mut(&mut self) -> &mut CallProfile {
        if let Self::Call(prof) = self {
            prof
        } else {
            panic!("Cannot unwrap EdgeTimings::{} as EdgeTimings::Call", self.variant());
        }
    }

    /// Returns the timing of the entire edge.
    /// 
    /// This is defined for all Edge types, so should not fail.
    /// 
    /// # Returns
    /// A reference to the internal Timing that represents the runtime of Edge as a whole.
    #[inline]
    pub fn edge_timing(&self) -> &Timing {
        match self {
            EdgeTimings::Node(prof)   => &prof.edge,
            EdgeTimings::Linear(prof) => &prof.edge,
            EdgeTimings::Join(prof)   => &prof.edge,
            EdgeTimings::Call(prof)   => &prof.edge,

            EdgeTimings::Other(timing) => timing,
        }
    }
    /// Returns the timing of the entire edge mutably.
    /// 
    /// This is defined for all Edge types, so should not fail.
    /// 
    /// # Returns
    /// A mutable reference to the internal Timing that represents the runtime of Edge as a whole.
    #[inline]
    pub fn edge_timing_mut(&mut self) -> &mut Timing {
        match self {
            EdgeTimings::Node(prof)   => &mut prof.edge,
            EdgeTimings::Linear(prof) => &mut prof.edge,
            EdgeTimings::Join(prof)   => &mut prof.edge,
            EdgeTimings::Call(prof)   => &mut prof.edge,

            EdgeTimings::Other(timing) => timing,
        }
    }
}
impl From<Timing> for EdgeTimings {
    #[inline]
    fn from(value: Timing) -> Self { EdgeTimings::Other(value) }
}

/// Contains timings for executing a Node Edge (:()
#[derive(Clone, Deserialize, Message, Serialize)]
pub struct NodeProfile {
    /// The time it takes to execute the entire edge.
    #[prost(tag = "1", required, message)]
    pub edge : Timing,

    /// The time it takes to prepare the call in the VM.
    #[prost(tag = "2", required, message)]
    pub pre  : Timing,
    /// The time it takes to do the external call itself.
    #[prost(tag = "3", required, message)]
    pub exec : Timing,
    /// The time it takes to process the result of the external call.
    #[prost(tag = "4", required, message)]
    pub post : Timing,
}
impl NodeProfile {
    /// Constructor for the NodeProfile that intializes all timings to be unset.
    /// 
    /// # Returns
    /// A new NodeProfile instance.
    #[inline]
    pub fn new() -> Self {
        Self {
            edge : Timing::new(),

            pre  : Timing::new(),
            exec : Timing::new(),
            post : Timing::new(),
        }
    }
}
impl AsRef<NodeProfile> for NodeProfile {
    #[inline]
    fn as_ref(&self) -> &Self { self }
}
impl<'de> Profile<'de> for NodeProfile {}

/// Contains timings for executing a Linear edge. The special case about this one is that we record per-instruction timings.
#[derive(Clone, Deserialize, Message, Serialize)]
pub struct LinearProfile {
    /// The time it takes to execute the entire edge.
    #[prost(tag = "1", required, message)]
    pub edge   : Timing,
    /// The time it takes to execute each instruction.
    #[prost(tag = "2", repeated, message)]
    pub instrs : Vec<Timing>,
}
impl LinearProfile {
    /// Constructor for the LinearProfile that intializes all timings to be unset.
    /// 
    /// # Returns
    /// A new LinearProfile instance.
    #[inline]
    pub fn new() -> Self {
        Self {
            edge   : Timing::new(),
            instrs : vec![],
        }
    }
}
impl AsRef<LinearProfile> for LinearProfile {
    #[inline]
    fn as_ref(&self) -> &Self { self }
}
impl<'de> Profile<'de> for LinearProfile {}

/// Contains timings for executing a Join edge. Effectively contains the timings of the branches in addition to its own timing.
#[derive(Clone, Deserialize, Message, Serialize)]
pub struct JoinProfile {
    /// The time it takes to execute the entire edge.
    #[prost(tag = "1", required, message)]
    pub edge     : Timing,
    /// The time it takes for each branch to execute. It's given as a ThreadProfile to also be able to get timing results of those branches.
    #[prost(tag = "2", repeated, message)]
    pub branches : Vec<ThreadProfile>,
}
impl JoinProfile {
    /// Constructor for the JoinProfile that intializes all timings to be unset.
    /// 
    /// # Returns
    /// A new JoinProfile instance.
    #[inline]
    pub fn new() -> Self {
        Self {
            edge : Timing::new(),

            branches : vec![],
        }
    }
}
impl AsRef<JoinProfile> for JoinProfile {
    #[inline]
    fn as_ref(&self) -> &Self { self }
}
impl<'de> Profile<'de> for JoinProfile {}

/// Contains timings for executing a Call edge. This typically provides the calling edge times itself only, unless we are calling a builtin.
#[derive(Clone, Deserialize, Message, Serialize)]
pub struct CallProfile {
    /// The name of the function we are calling.
    #[prost(tag = "1", required, string)]
    pub name : String,

    /// The time it takes to execute the entire edge.
    #[prost(tag = "2", required, message)]
    pub edge    : Timing,
    /// The time it takes for the builtin to complete, if any.
    #[prost(tag = "3", optional, message)]
    pub builtin : Option<Timing>,
}
impl CallProfile {
    /// Constructor for the CallProfile that intializes all timings to be unset.
    /// 
    /// # Arguments
    /// - `name`: The name of the function that we're calling.
    /// 
    /// # Returns
    /// A new CallProfile instance.
    #[inline]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name : name.into(),

            edge    : Timing::new(),
            builtin : None,
        }
    }
}
impl AsRef<CallProfile> for CallProfile {
    #[inline]
    fn as_ref(&self) -> &Self { self }
}
impl<'de> Profile<'de> for CallProfile {}
