//  SPEC.rs
//    by Lut99
// 
//  Created:
//    20 Oct 2022, 14:17:30
//  Last edited:
//    06 Nov 2022, 19:53:46
//  Auto updated?
//    Yes
// 
//  Description:
//!   Defines (public) interfaces and structs for the `brane-ast` crate.
// 

use brane_dsl::{DataType, TextRange};
use brane_dsl::data_type::FunctionSignature;

use crate::state::{ClassState, FunctionState, TableList, TableState, VarState};


/***** LIBRARY *****/
/// Defines the builtin functions that exist in BraneScript.
#[derive(Clone, Copy, Debug)]
pub enum BuiltinFunctions {
    /// The print-function, which prints some text to stdout.
    Print,
    /// The println-function, which does the same as `Print` but now with a newline appended to the text.
    PrintLn,

    /// The len-function, which returns the length of an array.
    Len,

    /// The commit_builtin-function, which turns an IntermediateResult into a Data.
    CommitResult,
}

impl BuiltinFunctions {
    /// Returns the identifier of this builtin function.
    #[inline]
    pub fn name(&self) -> &'static str {
        use BuiltinFunctions::*;
        match self {
            Print   => "print",
            PrintLn => "println",

            Len => "len",

            CommitResult => "commit_result",
        }
    }

    /// Returns the signature of this specific builtin.
    #[inline]
    pub fn signature(&self) -> FunctionSignature {
        use BuiltinFunctions::*;
        match self {
            Print   => FunctionSignature::new(vec![ DataType::String ], DataType::Void),
            PrintLn => FunctionSignature::new(vec![ DataType::String ], DataType::Void),

            Len => FunctionSignature::new(vec![ DataType::Array(Box::new(DataType::Any)) ], DataType::Integer),

            CommitResult => FunctionSignature::new(vec![ DataType::String, DataType::Class(BuiltinClasses::IntermediateResult.name().into()) ], DataType::Class(BuiltinClasses::Data.name().into())),
        }
    }



    /// Returns an array with all the builtin functions in it.
    #[inline]
    pub fn all() -> [ Self; 4 ] { [ Self::Print, Self::PrintLn, Self::Len, Self::CommitResult ] }

    /// Returns an Array with all of the builtin functions but already casted to FunctionStates.
    #[inline]
    pub fn all_into_state() -> [ FunctionState; 4 ] { [ Self::Print.into(), Self::PrintLn.into(), Self::Len.into(), Self::CommitResult.into() ] }
}

impl From<BuiltinFunctions> for FunctionState {
    #[inline]
    fn from(value: BuiltinFunctions) -> Self {
        Self {
            name      : value.name().into(),
            signature : value.signature(),

            class_name : None,

            table : TableState::none(),
            range : TextRange::none(),
        }
    }
}



/// Defines the builtin classes that exist in BraneScript.
#[derive(Clone, Copy, Debug)]
pub enum BuiltinClasses {
    /// The data-class.
    Data,
    /// The intermediate-result-class.
    IntermediateResult,
}

impl BuiltinClasses {
    /// Returns the identifier of this builtin class.
    #[inline]
    pub fn name(&self) -> &'static str {
        use BuiltinClasses::*;
        match self {
            Data               => "Data",
            IntermediateResult => "IntermediateResult",
        }
    }

    /// Returns a list of all properties (as `VarState`s) in this builtin class.
    #[inline]
    pub fn props(&self) -> Vec<VarState> {
        use BuiltinClasses::*;
        match self {
            Data               => vec![ VarState{ name: "name".into(), data_type: DataType::String, function_name: None, class_name: Some(self.name().into()), range: TextRange::none() } ],
            IntermediateResult => vec![ VarState{ name: "path".into(), data_type: DataType::String, function_name: None, class_name: Some(self.name().into()), range: TextRange::none() } ],
        }
    }

    /// Returns a list of all methods (as `FunctioNState`s) in this builtin class.
    #[inline]
    pub fn methods(&self) -> Vec<FunctionState> {
        use BuiltinClasses::*;
        match self {
            Data               => vec![],
            IntermediateResult => vec![],
        }
    }



    /// Returns an array with all the builtin classes in it.
    #[inline]
    pub fn all() -> [ Self; 2 ] { [ Self::Data, Self::IntermediateResult ] }

    /// Returns an Array with all of the builtin functions but already casted to FunctionStates.
    /// 
    /// # Arguments
    /// - `funcs`: The list of function states to use for declaring new methods, if any.
    #[inline]
    pub fn all_into_state(funcs: &mut TableList<FunctionState>) -> [ ClassState; 2 ] { [ Self::Data.into_state(funcs), Self::IntermediateResult.into_state(funcs) ] }



    /// Creates a new ClassState for this BuiltinClasses, where we define the functions in the given TableList of functions.
    /// 
    /// # Arguments
    /// - `funcs`: The TableList of functions where to declare the new ones.
    /// 
    /// # Returns
    /// A new ClassState instance.
    #[inline]
    pub fn into_state(&self, funcs: &mut TableList<FunctionState>) -> ClassState {
        ClassState {
            name    : self.name().into(),
            props   : self.props(),
            methods : self.methods().into_iter().map(|f| funcs.push(f)).collect(),

            package_name    : None,
            package_version : None,

            range : TextRange::none(),
        }
    }
}
