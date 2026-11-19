package vendor.oplus.hardware.urcc;

// UrccRequestData — one resource state sample returned by the URCC HAL.
//
// Field order is the wire order and must not change.
parcelable UrccRequestData {
    int res_Id;
    int event_Id;
    String res_Value;
    String res_Path;
    String desc;
}
