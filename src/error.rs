// error reporting!
// this is a particularly thorny problem. Rust's built-in error handling (assisted with crates like anyhow) is great for debugging interpreters,
// but it's not great for debugging interpreted code.
// this is my attempt to solve that problem.


/* error outputs look like:
== ERROR ==

Include Error at file.txt:3:10
  Failed to include `includer.txt`

Caused By

Parsing Error at includer.txt:17:19:
  Expected `,` or `]`, found `)`.
*/

use crate::utility::*;


pub struct Error {
    pub span : Span,
    pub tp : String,
    pub reason : String,
    pub cause : Option<Box<Error>>
}


impl Error {
    pub fn expected(one_of : &[TokenType], got : Token) -> Error {
        let mut reason = "Expected ".to_string();
        for i in 0..one_of.len() {
            reason += &one_of[i].to_string();
            if one_of.len() >= 2 && i == one_of.len() - 2 {
                reason += " or ";
            }
            else if one_of.len() >= 2 && i < one_of.len() - 2 {
                reason += ", ";
            }
        }
        reason += ", got ";
        reason += &got.tp.to_string();
        Error {
            span : got.span,
            tp : "Parsing".to_string(),
            reason,
            cause : None
        }
    }

    pub fn unexpected_eof(at : Span) -> Error {
        Error {
            span : at,
            tp : "Parsing".to_string(),
            reason : "Unexpected End-Of-File".to_string(),
            cause : None
        }
    }

    pub fn unexpected_char(c : char, at : Span) -> Error {
        Error {
            span : at,
            tp : "Parsing".to_string(),
            reason : format!("Unexpected Character {}", c),
            cause : None
        }
    }

    pub fn expected_abstract(e : impl ToString + std::fmt::Display, at : Span) -> Error {
        Error {
            span : at,
            tp : "Parsing".to_string(),
            reason : format!("Expected {}", e),
            cause : None
        }
    }

    pub fn bad_argument(at : Token) -> Error {
        Error {
            span : at.span,
            tp : "Parsing".to_string(),
            reason : "Invalid Argument".to_string(),
            cause : None
        }
    }

    pub fn wrap<T>(mut self, around : SitixResult<T>) -> SitixResult<T> {
        if let Err(e) = around {
            self.cause = Some(Box::new(e));
            Err(self)
        }
        else {
            around
        }
    }

    fn debug_inner(&self, f : &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (line, col) = self.span.get_line_col();
        write!(f, "{} Error at {}:{}:{}\n", self.tp, self.span.filename, line, col)?;
        write!(f, "  {}", self.reason)?;
        if let Some(cause) = &self.cause {
            write!(f, "\n\nCaused By\n\n")?;
            cause.debug_inner(f)?;
        }
        Ok(())
    }
}


impl std::fmt::Debug for Error {
    fn fmt(&self, f : &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "== ERROR ==\n\n")?;
        self.debug_inner(f)?;
        Ok(())
    }
}


impl std::fmt::Display for Error {
    fn fmt(&self, f : &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Self as std::fmt::Debug>::fmt(self, f)
    }
}


impl std::error::Error for Error {}


pub type SitixResult<T> = Result<T, Error>;


// just the juicy bits
// a PartialError can tell you what happened, but not where or why
// meant to be returned by functions that don't have access to localization context (a Token-at-fault), and `weld`ed
// by functions that do.
#[derive(Debug)]
pub struct PartialError {
    pub tp : String,
    pub reason : String
}


impl PartialError {
    pub fn weld(self, span : Span) -> Error {
        Error {
            tp : self.tp,
            reason : self.reason,
            span,
            cause : None
        }
    }

    pub fn invalid_type(expected : impl std::fmt::Display, got : impl std::fmt::Display) -> PartialError {
        PartialError {
            tp : "Runtime".to_string(),
            reason : format!("Expected a {}, got a {}", expected, got)
        }
    }

    pub fn undefined_symbol() -> PartialError { // TODO: include the name of the affected symbol here
        PartialError {
            tp : "Runtime".to_string(),
            reason : "Undefined symbol".to_string()
        }
    }

    pub fn invalid_index(index : String) -> PartialError {
        PartialError {
            tp : "Runtime".to_string(),
            reason : format!("Invalid index {}", index)
        }
    }
}


pub type SitixPartialResult<T> = Result<T, PartialError>;
