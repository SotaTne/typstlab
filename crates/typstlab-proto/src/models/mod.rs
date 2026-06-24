/*
 *
 * model/は非推奨
 * 段階的に消していく。
 * 以降はpaht,storeなどを使っていくこと
 *
 */

pub mod identity;
pub mod lifecycle;
pub mod location;

pub use identity::{Entity, Model};
pub use lifecycle::{Creatable, Loadable, Loaded};
pub use location::{Locatable, Location, Remote};
