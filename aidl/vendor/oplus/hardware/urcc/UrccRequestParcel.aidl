package vendor.oplus.hardware.urcc;

import vendor.oplus.hardware.urcc.IUrccCallback;
import vendor.oplus.hardware.urcc.UrccRequestData;

// UrccRequestParcel — the request the client sends to the URCC HAL.
//
// Field order is the wire order and must not change: the framework's Java stub
// (RE/framework/oplus-framework/vendor/oplus/hardware/urcc/UrccRequestParcel.smali)
// writes and reads exactly this sequence.
parcelable UrccRequestParcel {
    int type;
    UrccRequestData[] mUrccRequestData;
    boolean screenoff_support;
    int prio;
    int callingPID;
    int callingUID;
    long duration;
    String[] mParams;
    int handle;
    IUrccCallback urccCallback;
    String pkgName;
    String identity;
    byte[] otherData;
}
