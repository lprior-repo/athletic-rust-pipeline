mod evidence;
mod fixture;
mod resources_and_replay;
mod selection;
mod temporal;

use std::error::Error;
type TestResult<T = ()> = Result<T, Box<dyn Error>>;

mod material_admission;

mod material_branches;
