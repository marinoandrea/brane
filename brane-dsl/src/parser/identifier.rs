//  IDENTIFIER.rs
//    by Lut99
// 
//  Created:
//    10 Aug 2022, 17:13:42
//  Last edited:
//    25 Aug 2022, 11:22:49
//  Auto updated?
//    Yes
// 
//  Description:
//!   Defines the function(s) that parse identifiers.
// 

use std::num::NonZeroUsize;

use nom::combinator as comb;
use nom::error::{ContextError, ParseError};
use nom::{IResult, Parser};

use super::wrap_pp;
use super::ast::Identifier;
use crate::scanner::{Token, Tokens};
use crate::tag_token;


/***** LIBRARY *****/
/// Parses an iodentifier Token to an Identifier node in the AST.
///
/// # Arguments
/// - `input`: The list of tokens to parse from.
/// 
/// # Returns
/// The remaining list of tokens and the parsed Identifier if there was anything to parse. Otherwise, a `nom::Error` is returned (which may be a real error or simply 'could not parse').
pub fn parse<'a, E: ParseError<Tokens<'a>> + ContextError<Tokens<'a>>>(input: Tokens<'a>) -> IResult<Tokens, Identifier, E> {
    wrap_pp!(
        comb::map(
            tag_token!(Token::Ident),
            |t| Identifier::new(t.tok[0].as_string(), t.tok[0].inner().into()),
        ).parse(input),
    "IDENTIFIER")
}
