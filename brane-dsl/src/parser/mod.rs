//  MOD.rs
//    by Lut99
// 
//  Created:
//    18 Aug 2022, 09:48:12
//  Last edited:
//    21 Sep 2022, 14:04:03
//  Auto updated?
//    Yes
// 
//  Description:
//!   Defines nom-functions that parse token streams to a temporary AST
//!   that is suited for analysing in `brane-ast`.
// 

// Declare private modules
mod expression;
mod identifier;
mod instance;
mod literal;
mod operator;
// mod pattern;

// Declare public modules
pub mod ast;
pub mod bakery;
pub mod bscript;


// Declare macros
/// Wrapper around enter_pp that is only enabled if print_parser_path is enabled
#[cfg(feature = "print_parser_path")]
macro_rules! enter_pp {
    ($($arg:tt)+) => {
        crate::enter_pp!($($arg)+)
    };
}
/// Wrapper around enter_pp that is only enabled if print_parser_path is enabled
#[cfg(not(feature = "print_parser_path"))]
macro_rules! enter_pp {
    ($($arg:tt)+) => {};
}
pub(crate) use enter_pp;

/// Wrapper around exit_pp that is only enabled if print_parser_path is enabled
#[cfg(feature = "print_parser_path")]
macro_rules! exit_pp {
    ($res:expr, $($arg:tt)+) => {
        crate::exit_pp!($res, $($arg)+)
    };
}
/// Wrapper around exit_pp that is only enabled if print_parser_path is enabled
#[cfg(not(feature = "print_parser_path"))]
macro_rules! exit_pp {
    ($res:expr, $($arg:tt)+) => {
        $res
    };
}
pub(crate) use exit_pp;

/// Wrapper around wrap_pp that is only enabled if print_parser_path is enabled
#[cfg(feature = "print_parser_path")]
macro_rules! wrap_pp {
    ($res:expr, $($arg:tt)+) => {
        crate::wrap_pp!($res, $($arg)+)
    };
}
/// Wrapper around wrap_pp that is only enabled if print_parser_path is enabled
#[cfg(not(feature = "print_parser_path"))]
macro_rules! wrap_pp {
    ($res:expr, $($arg:tt)+) => {
        $res
    };
}
pub(crate) use wrap_pp;


/// Defines a macro that parses the given token from the given stream of tokens.
#[macro_export]
macro_rules! tag_token (
    (Token::$variant:ident) => (
        move |i: Tokens<'a>| {
            use nom::{Err, error_position, Needed, try_parse, take};
            use nom::error::ErrorKind;

            if i.tok.is_empty() {
                match stringify!($variant) {
                    "Dot" => Err(Err::Error(E::from_char(i, '.'))),
                    "Colon" => Err(Err::Error(E::from_char(i, ':'))),
                    "Comma" => Err(Err::Error(E::from_char(i, ','))),
                    "LeftBrace" => Err(Err::Error(E::from_char(i, '{'))),
                    "LeftBracket" => Err(Err::Error(E::from_char(i, '['))),
                    "LeftParen" => Err(Err::Error(E::from_char(i, '('))),
                    "RightBrace" => Err(Err::Error(E::from_char(i, '}'))),
                    "RightBracket" => Err(Err::Error(E::from_char(i, ']'))),
                    "RightParen" => Err(Err::Error(E::from_char(i, ')'))),
                    "Semicolon" => Err(Err::Error(E::from_char(i, ';'))),
                    _ => {
                        Err(Err::Error(error_position!(i, ErrorKind::Eof)))
                    }
                }
            } else {
                let (i1, t1) = try_parse!(i, take!(1));

                if t1.tok.is_empty() {
                    Err(Err::Incomplete(Needed::Size(NonZeroUsize::new(1).unwrap())))
                } else {
                    if let Token::$variant(_) = t1.tok[0] {
                        Ok((i1, t1))
                    } else {
                        match stringify!($variant) {
                            "Dot" => Err(Err::Error(E::from_char(i, '.'))),
                            "Colon" => Err(Err::Error(E::from_char(i, ':'))),
                            "Comma" => Err(Err::Error(E::from_char(i, ','))),
                            "LeftBrace" => Err(Err::Error(E::from_char(i, '{'))),
                            "LeftBracket" => Err(Err::Error(E::from_char(i, '['))),
                            "LeftParen" => Err(Err::Error(E::from_char(i, '('))),
                            "RightBrace" => Err(Err::Error(E::from_char(i, '}'))),
                            "RightBracket" => Err(Err::Error(E::from_char(i, ']'))),
                            "RightParen" => Err(Err::Error(E::from_char(i, ')'))),
                            "Semicolon" => Err(Err::Error(E::from_char(i, ';'))),
                            _ => {
                                Err(Err::Error(error_position!(i, ErrorKind::Tag)))
                            }
                        }
                    }
                }
            }
        }
    );
);
