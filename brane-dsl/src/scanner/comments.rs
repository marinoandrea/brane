//  COMMENTS.rs
//    by Lut99
// 
//  Created:
//    25 Aug 2022, 11:08:56
//  Last edited:
//    03 Nov 2022, 19:21:32
//  Auto updated?
//    Yes
// 
//  Description:
//!   Defines a few scanning functions that parse comments.
// 

use nom::error::{ContextError, ParseError};
use nom::{branch, bytes::complete as bc, combinator as comb, sequence as seq};
use nom::{IResult, Parser};

use super::{wrap_pp, Span};
use super::tokens::Token;


/***** SCANNING FUNCTIONS *****/
/// Parses a single-line comment off the top of the given input.
/// 
/// # Arguments
/// - `input`: The input text to scan.
/// 
/// # Returns
/// The remaining tokens and a `Token::None`, representing that we did not really parse useful information.
/// 
/// # Errors
/// This function errors if we could not parse a comment.
pub fn single_line_comment<'a, E: ParseError<Span<'a>> + ContextError<Span<'a>>>(
    input: Span<'a>
) -> IResult<Span<'a>, Token, E> {
    wrap_pp!(
        comb::value(Token::None, seq::pair(
            bc::tag("//"),
            seq::terminated(
                comb::opt(bc::is_not("\n")),
                branch::alt((
                    bc::tag("\n"),
                    comb::eof,
                )),
            )
        )).parse(input),
    "SINGLE-LINE COMMENT")
}

/// Parses a multi-line comment off the top of the given input.
/// 
/// # Arguments
/// - `input`: The input text to scan.
/// 
/// # Returns
/// The remaining tokens and a `Token::None`, representing that we did not really parse useful information.
/// 
/// # Errors
/// This function errors if we could not parse a comment.
pub fn multi_line_comment<'a, E: ParseError<Span<'a>> + ContextError<Span<'a>>>(
    input: Span<'a>
) -> IResult<Span<'a>, Token, E> {
    wrap_pp!(
        comb::value(
            Token::None,
            seq::tuple((bc::tag("/*"), comb::cut(seq::pair(bc::take_until("*/"), bc::tag("*/"))))),
        )
        .parse(input),
    "MULTI-LINE COMMENT")
}





/***** LIBRARY *****/
/// Scans a comment from the top of the given input.
/// 
/// # Arguments
/// - `input`: The input text to scan.
/// 
/// # Returns
/// The remaining tokens and a `Token::None`, representing that we did not really parse useful information.
/// 
/// # Errors
/// This function errors if we could not parse a comment.
pub fn parse<'a, E: ParseError<Span<'a>> + ContextError<Span<'a>>>(input: Span<'a>) -> IResult<Span<'a>, Token, E> {
    // println!("COMMENTS")
    wrap_pp!(
        branch::alt((single_line_comment, multi_line_comment)).parse(input),
    "COMMENT")
}
