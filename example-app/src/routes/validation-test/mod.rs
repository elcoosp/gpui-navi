use navi_router::RouteDef;
#[cfg(feature = "garde")]
pub mod garde;
pub mod index;
#[cfg(feature = "valico")]
pub mod valico;
#[cfg(feature = "validator")]
pub mod validator;
#[cfg(feature = "validify")]
pub mod validify;
