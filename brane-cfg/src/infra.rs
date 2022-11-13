//  INFRA.rs
//    by Lut99
// 
//  Created:
//    04 Oct 2022, 11:04:33
//  Last edited:
//    02 Nov 2022, 16:36:21
//  Auto updated?
//    Yes
// 
//  Description:
//!   Implements a more up-to-date version of the infrastructure document.
// 

use std::collections::HashMap;
use std::fs::File;

use serde::Deserialize;

pub use crate::errors::InfraFileError as Error;
use crate::spec::{InfraLocation, InfraPath};
use crate::secrets::resolve_secrets;


/***** LIBRARY *****/
/// Defines a "handle" to the document that contains the Brane instance layout.
/// 
/// It is recommended to only load when used, to allow system admins to update the file during runtime.
#[derive(Debug, Deserialize)]
pub struct InfraFile {
    /// The address of the API endpoint.
    registry           : String,
    /// The address of the Docker registry endpoint.
    container_registry : String,

    /// The map of locations (mapped by ID).
    locations : HashMap<String, InfraLocation>,
}

impl InfraFile {
    /// Reads the `infra.yml` file at the given path to an InfraFile.
    /// 
    /// # Arguments
    /// - `infra_path`: The InfraPath file that contains the paths we need.
    /// 
    /// # Returns
    /// A new InfraFile instance.
    /// 
    /// # Errors
    /// This function fails if we could either not read the file or the file was not valid YAML.
    pub fn from_path(infra_path: impl AsRef<InfraPath>) -> Result<Self, Error> {
        let infra_path : &InfraPath = infra_path.as_ref();

        // Open the file
        let handle: File = match File::open(&infra_path.infra) {
            Ok(handle) => handle,
            Err(err)   => { return Err(Error::FileOpenError { path: infra_path.infra.clone(), err }); },  
        };

        // Run it through serde, done
        let mut result: Self = match serde_yaml::from_reader(handle) {
            Ok(locs) => locs,
            Err(err) => { return Err(Error::FileParseError { path: infra_path.infra.clone(), err }); },
        };

        // Resolve the secrets
        match resolve_secrets(&mut result.locations, &infra_path.secrets) {
            Ok(_)    => Ok(result),
            Err(err) => Err(Error::SecretsResolveError{ path: infra_path.infra.clone(), err }),
        }
    }



    /// Returns the address of the main central node registry.
    #[inline]
    pub fn registry(&self) -> &str { &self.registry }

    /// Returns the address of the main central node Docker registry.
    #[inline]
    pub fn container_registry(&self) -> &str { &self.container_registry }

    /// Returns the metadata for the location with the given name.
    /// 
    /// # Arguments
    /// - `name`: The name of the location to retrieve.
    /// 
    /// # Returns
    /// The InfraLocation of the location that was referenced by the name, or else `None` if it didn't exist.
    #[inline]
    pub fn get(&self, name: impl AsRef<str>) -> Option<&InfraLocation> {
        self.locations.get(name.as_ref())
    }



    /// Returns an iterator-by-reference over the internal map.
    #[inline]
    pub fn iter(&self) -> std::collections::hash_map::Iter<String, InfraLocation> { self.into_iter() }

    /// Returns a muteable iterator-by-reference over the internal map.
    #[inline]
    pub fn iter_mut(&mut self) -> std::collections::hash_map::IterMut<String, InfraLocation> { self.into_iter() }

}

impl IntoIterator for InfraFile {
    type Item     = (String, InfraLocation);
    type IntoIter = std::collections::hash_map::IntoIter<String, InfraLocation>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.locations.into_iter()
    }
}
impl<'a> IntoIterator for &'a InfraFile {
    type Item     = (&'a String, &'a InfraLocation);
    type IntoIter = std::collections::hash_map::Iter<'a, String, InfraLocation>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.locations.iter()
    }
}
impl<'a> IntoIterator for &'a mut InfraFile {
    type Item     = (&'a String, &'a mut InfraLocation);
    type IntoIter = std::collections::hash_map::IterMut<'a, String, InfraLocation>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.locations.iter_mut()
    }
}
