#[path = "../generated/stxn.io.rs"]
mod stxn;
#[path = "../generated/stxn.io.serde.rs"]
mod stxn_json;
pub use stxn::*;

#[path = "../generated/vamp.fun.rs"]
mod vamp_fun;
#[path = "../generated/vamp.fun.serde.rs"]
mod vamp_fun_json;
pub use vamp_fun::*;
