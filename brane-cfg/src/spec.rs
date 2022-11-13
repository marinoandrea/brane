//  SPEC.rs
//    by Lut99
// 
//  Created:
//    04 Oct 2022, 11:42:49
//  Last edited:
//    01 Nov 2022, 11:02:09
//  Auto updated?
//    Yes
// 
//  Description:
//!   Defines (public) interfaces and structs for the `brane-cfg` crate.
// 

use std::path::PathBuf;

use serde::Deserialize;


/***** LIBRARY *****/
/// Defines a convenient pair for referencing both the infra file and its associated secrets file.
#[derive(Clone, Debug)]
pub struct InfraPath {
    /// The path to the infrastructure file.
    pub infra   : PathBuf,
    /// The path to the secrets file.
    pub secrets : PathBuf,
}

impl InfraPath {
    /// Constructor for the InfraPath that creates from a pair of paths.
    /// 
    /// # Arguments
    /// - `infra`: The path to the infrastructure file itself.
    /// - `secrets`: The path to the secrets file itself.
    /// 
    /// # Returns
    /// A new instance of the InfraPath for those files.
    #[inline]
    pub fn new(infra: impl Into<PathBuf>, secrets: impl Into<PathBuf>) -> Self {
        Self {
            infra   : infra.into(),
            secrets : secrets.into()
        }
    }
}

impl AsRef<InfraPath> for InfraPath {
    #[inline]
    fn as_ref(&self) -> &InfraPath {
        self
    }
}
impl From<&InfraPath> for InfraPath {
    #[inline]
    fn from(value: &InfraPath) -> Self {
        value.clone()
    }
}
impl From<&mut InfraPath> for InfraPath {
    #[inline]
    fn from(value: &mut InfraPath) -> Self {
        value.clone()
    }
}

impl<P1, P2> From<(P1, P2)> for InfraPath
where
    P1: Into<PathBuf>,
    P2: Into<PathBuf>,
{
    #[inline]
    fn from(value: (P1, P2)) -> Self {
        Self::new(value.0, value.1)
    }
}
impl<P1, P2> From<&(P1, P2)> for InfraPath
where
    P1: Clone + Into<PathBuf>,
    P2: Clone + Into<PathBuf>,
{
    #[inline]
    fn from(value: &(P1, P2)) -> Self {
        Self::from((value.0.clone(), value.1.clone()))
    }
}



/// Defines a single Location in the InfraFile.
#[derive(Clone, Debug, Deserialize)]
pub struct InfraLocation {
    /// Defines a more human-readable name for the location.
    pub name     : String,
    /// The address of the delegate to connect to.
    pub delegate : String,
    /// The address of the local registry to query for locally available packages, datasets and more.
    pub registry : String,
}
