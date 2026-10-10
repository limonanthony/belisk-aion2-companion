pub mod notes;
pub mod plan;

use crate::Result;

pub fn annotate_failure<T>(result: Result<T>) -> Result<T> {
    if let Err(error) = &result {
        if std::env::var_os("GITHUB_ACTIONS").is_some() {
            println!("::error::{error}");
        }
    }
    result
}
