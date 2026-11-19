// The cross-platform half of the OPlus URCC HAL.
//
// The resource table, the node mapping and the compatibility layer for
// resources without a direct node live here so they can be unit-tested on the
// host. The binder-facing half (wire.rs / hal.rs) is an Android-only binary
// module in src/bin/urcc_service.rs.

pub mod core;
pub mod io_shim;
pub mod table;
