// The cross-platform half of the OPlus URCC HAL port.
//
// The resource table, the MediaTek node mapping and the compatibility /
// simulation layer for resources without an MTK node live here so they can be
// unit-tested on the host. The binder-facing half (wire.rs / hal.rs) is an
// Android-only binary module in src/bin/urcc_service.rs.

pub mod core;
pub mod io_shim;
pub mod table;
