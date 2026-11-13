package vendor.oplus.hardware.urcc;

// UrccRequestData — one resource state sample returned by the URCC HAL.
//
// Field order is the wire order and must not change: the framework's Java stub
// (RE/framework/oplus-framework/vendor/oplus/hardware/urcc/UrccRequestData.smali)
// writes and reads exactly this sequence.
parcelable UrccRequestData {
    int res_Id;
    int event_Id;
    String res_Value;
    String res_Path;
    String desc;
}
