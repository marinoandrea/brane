//  TOOLS.rs
//    by Lut99
// 
//  Created:
//    31 Oct 2022, 13:59:36
//  Last edited:
//    09 Nov 2022, 11:32:52
//  Auto updated?
//    Yes
// 
//  Description:
//!   Contains generic tools to use across the use-cases.
// 

use crate::errors::ExecuteError;


/***** LIBRARY *****/
/// Decodes the given base64 string to a normal string if it is UTF-8.
/// 
/// # Arguments
/// - `raw`: The encoded Base64 string to decode.
/// 
/// # Returns
/// The decoded string the raw Base64 text represents.
/// 
/// # Errors
/// This function errors if the given text was not valid UTF-8.
pub fn decode_base64(raw: impl AsRef<str>) -> Result<String, ExecuteError> {
    let raw: &str = raw.as_ref();

    // First, try to decode the raw base64
    let input: Vec<u8> = match base64::decode(raw) {
        Ok(bin)     => bin,
        Err(reason) => { return Err(ExecuteError::Base64DecodeError{ raw: raw.into(), err: reason }); }
    };

    // Next, try to decode the binary as UTF-8
    match String::from_utf8(input.clone()) {
        Ok(text)    => Ok(text),
        Err(reason) => Err(ExecuteError::Utf8DecodeError{ raw: String::from_utf8_lossy(&input).into(), err: reason }),
    }

    // We leave JSON for another day
}
