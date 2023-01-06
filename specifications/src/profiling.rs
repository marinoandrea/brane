//  PROFILING.rs
//    by Lut99
// 
//  Created:
//    06 Jan 2023, 11:47:00
//  Last edited:
//    06 Jan 2023, 14:42:46
//  Auto updated?
//    Yes
// 
//  Description:
//!   Contains some structs that we use to carry around profiling
//!   information.
// 

use std::fmt::Debug;
use std::time::{Duration, Instant};

use prost::Message;
use serde::{Deserialize, Serialize};
use serde::de::{Deserializer, Visitor};
use serde::ser::Serializer;


/***** HELPER MACROS *****/
/// A helper macro for immediately showing the timing from a string.
#[macro_export]
macro_rules! timing {
    ($raw:expr, $format_fn:ident) => {
        serde_json::from_str::<specifications::profiling::Timing>($raw).map(|t| format!("{}", t.$format_fn())).unwrap_or("<unparseable>".into())
    };
}





/***** AUXILLARY *****/
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
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Timing(Option<Instant>, Option<Instant>);

impl Default for Timing {
    #[inline]
    fn default() -> Self { Self::new() }
}
impl Timing {
    /// Constructor for the Timing that initializes it as empty.
    /// 
    /// # Returns
    /// A new Timing instance on which you have to call `Timing::start()` and `Timing::stop()` still.
    #[inline]
    pub fn new() -> Self { Self(None, None) }

    /// Constructor for the Timing that immediately starts timing.
    /// 
    /// # Returns
    /// A new Timing instance with the start time set to now. You still have to call `Timing::stop`.
    #[inline]
    pub fn new_start() -> Self { Self(Some(Instant::now()), None) }



    /// Starts the timing.
    /// 
    /// If it has been started already, simply overrides the start time with the current time.
    /// 
    /// Always resets the stop time to be unset.
    #[inline]
    pub fn start(&mut self) {
        self.0 = Some(Instant::now());
        self.1 = None;
    }

    /// Stops the timing.
    /// 
    /// If it has been stopped already, simply overrides the stop time with the current time.
    /// 
    /// # Panics
    /// This function will panic if `Timing::start()` has not yet been called.
    #[inline]
    pub fn stop(&mut self) {
        if self.0.is_none() { panic!("Cannot call `Timing::stop()` without calling `Timing::start()` first"); }
        self.1 = Some(Instant::now())
    }

    /// Returns a TimingGuard which will call `Timing::start()` when created and `Timing::stop()` when it is destroyed (i.e., goes out-of-scope).
    #[inline]
    pub fn guard(&mut self) -> TimingGuard { TimingGuard::new(self) }



    /// Returns whether this Timing has been successfully started and stopped (i.e., a time taken can be computed).
    #[inline]
    pub fn is_taken(&self) -> bool { self.0.is_some() && self.1.is_some() }

    /// Returns the time taken in milliseconds.
    /// 
    /// # Panics
    /// This function will panic if the timing is not successfully taken (i.e., either `Timing::start()` of `Timing::stop` has not been called).
    #[inline]
    pub fn elapsed_ms(&self) -> u128 {
        if let (Some(start), Some(stop)) = (self.0, self.1) {
            stop.duration_since(start).as_millis()
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
        if let (Some(start), Some(stop)) = (self.0, self.1) {
            stop.duration_since(start).as_micros()
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
        if let (Some(start), Some(stop)) = (self.0, self.1) {
            stop.duration_since(start).as_nanos()
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
        if let (Some(start), Some(stop)) = (self.0, self.1) {
            serializer.serialize_u64(stop.duration_since(start).as_nanos() as u64)
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

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value,E>
            where
                E:serde::de::Error,
            {
                // Take the current instant as start
                let start : Instant = Instant::now();
                // Add the elapsed time to find the end
                let end   : Instant = start + Duration::from_nanos(v);

                // Done
                Ok(Timing(Some(start), Some(end)))
            }
        }

        // Simply visit the timing
        deserializer.deserialize_u64(TimingVisitor)
    }
}
impl Message for Timing {
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: prost::bytes::BufMut,
        Self: Sized,
    {
        // Compute the elapsed time
        let elapsed_ns: u64 = if let (Some(start), Some(stop)) = (self.0, self.1) {
            stop.duration_since(start).as_nanos() as u64
        } else {
            panic!("Cannot serialize a Timing that is not yet taken (call `Timing::start()` and `Timing::stop()` first)");
        };

        // Encode it
        elapsed_ns.encode_raw(buf)
    }
    #[inline]
    fn encoded_len(&self) -> usize { std::mem::size_of::<u64>() }

    fn merge_field<B>(&mut self, tag: u32, wire_type: prost::encoding::WireType, buf: &mut B, ctx: prost::encoding::DecodeContext) -> Result<(), prost::DecodeError>
    where
        B: prost::bytes::Buf,
        Self: Sized,
    {
        // Get the number itself
        let mut elapsed_ns: u64 = 0;
        elapsed_ns.merge_field(tag, wire_type, buf, ctx)?;

        // Set the start & stop accordingly
        let start: Instant = Instant::now();
        self.0 = Some(start);
        self.1 = Some(start + Duration::from_nanos(elapsed_ns));

        // Done
        Ok(())
    }

    #[inline]
    fn clear(&mut self) { *self = Default::default(); }
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
