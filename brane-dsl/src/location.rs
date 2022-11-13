//  LOCATIONS.rs
//    by Lut99
// 
//  Created:
//    26 Aug 2022, 15:44:19
//  Last edited:
//    06 Sep 2022, 09:24:36
//  Auto updated?
//    Yes
// 
//  Description:
//!   Defines helpful enums and functions for location analysis.
// 

use std::collections::HashSet;
use std::mem;


/***** LIBRARY *****/
/// Defines a special type that presents a location.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Location(pub String);

impl From<String> for Location {
    #[inline]
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&String> for Location {
    #[inline]
    fn from(value: &String) -> Self {
        Self(value.clone())
    }
}

impl From<&str> for Location {
    #[inline]
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}

impl From<Location> for String {
    #[inline]
    fn from(value: Location) -> Self {
        value.0
    }
}

impl From<&Location> for String {
    #[inline]
    fn from(value: &Location) -> Self {
        value.0.clone()
    }
}



/// Defines an enum that says something about the range of the location.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AllowedLocations {
    /// Everything is allowed (policy-wise).
    All,
    /// There is a restricted set of locations.
    Exclusive(HashSet<Location>),
}

impl AllowedLocations {
    /// Computes the intersection between this AllowedLocations and the given one.
    /// 
    /// In other words, after running, this AllowedLocation contains only locations allowed by both.
    /// 
    /// # Arguments
    /// - `other`: The other AllowedLocations to take the intersection with.
    #[inline]
    pub fn intersection(&mut self, other: &mut AllowedLocations) {
        use AllowedLocations::*;
        match self {
            All                  => { mem::swap(self, other); },
            Exclusive(self_locs) => {
                match other.as_ref() {
                    All                   => {},
                    Exclusive(other_locs) => {
                        // Take the self_locs
                        let old_locs: HashSet<Location> = mem::take(self_locs);

                        // Add those only present in both
                        let mut res_locs: HashSet<Location> = HashSet::with_capacity(old_locs.len());
                        for l in old_locs {
                            if other_locs.contains(&l) { res_locs.insert(l); }
                        }
                        res_locs.shrink_to_fit();

                        // Set 'em
                        *self_locs = res_locs;
                    }
                }
            }
        }
    }



    /// Returns whether all locations are allowed right now.
    #[inline]
    pub fn is_all(&self) -> bool { if let AllowedLocations::All = self { true } else { false } }

    /// Returns whether only specific locations are allowed right now.
    #[inline]
    pub fn is_exclusive(&self) -> bool { if let AllowedLocations::Exclusive(_) = self { true } else { false } }

    /// Returns whether _no_ location is still allowed right now.
    #[inline]
    pub fn is_empty(&self) -> bool {
        match self {
            AllowedLocations::All             => false,
            AllowedLocations::Exclusive(locs) => locs.is_empty(),
        }
    }
}

impl AsRef<AllowedLocations> for AllowedLocations {
    #[inline]
    fn as_ref(&self) -> &AllowedLocations { self }
}

impl From<Location> for AllowedLocations {
    #[inline]
    fn from(value: Location) -> Self {
        Self::Exclusive(HashSet::from([ value ]))
    }
}
