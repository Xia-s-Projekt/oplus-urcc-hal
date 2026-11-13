package vendor.oplus.hardware.urcc;

import vendor.oplus.hardware.urcc.IUrccCallback;
import vendor.oplus.hardware.urcc.UrccRequestData;
import vendor.oplus.hardware.urcc.UrccRequestParcel;

// IUrcc — Oplus Unified Resource Control / Coordination.
//
// Recovered from three independent sources that agree:
//   * the framework's Java stub
//     (RE/framework/oplus-framework/vendor/oplus/hardware/urcc/IUrcc.smali and
//      IUrcc$Stub$Proxy.smali, which fix both the signatures and the
//      transaction codes),
//   * the shipped AIDL backend
//     (vendor.oplus.hardware.urcc-V1-ndk_platform.so, IUrccDefault::*),
//   * the shipped service
//     (odm/bin/hw/vendor.oplus.hardware.urcc-service).
//
// The method order below IS the transaction order (AIDL numbers methods from 1
// in declaration order). The framework stub uses:
//   1 urccInit              8 urccPropertySet
//   2 urccResCtlRequest     9 urccThermalListeningRegister
//   3 urccResCtlRelease    10 urccThermalListeningUnRegister
//   4 urccResStateRequest  11 uahNotifyExt
//   5 urccResListeningRegister 12 setRelatedSysInfo
//   6 urccResListeningUnRegister 13 urccRuleCtl
//   7 urccPropertyGet      14 uahResCtlRequestBypass
// The reserved built-ins are getInterfaceVersion (0xffffff) and
// getInterfaceHash (0xfffffe).
interface IUrcc {
    void urccInit();

    int urccResCtlRequest(in UrccRequestParcel request);

    int urccResCtlRelease(int handle);

    UrccRequestData[] urccResStateRequest(in UrccRequestParcel request);

    int urccResListeningRegister(in UrccRequestParcel request, IUrccCallback callback);

    int urccResListeningUnRegister(int handle);

    String urccPropertyGet(String key);

    int urccPropertySet(String key, String value);

    int urccThermalListeningRegister(in int[] types, IUrccCallback callback);

    int urccThermalListeningUnRegister(int handle);

    int uahNotifyExt(int src, int type, in int[] data);

    int setRelatedSysInfo(int type, in byte[] info);

    int urccRuleCtl(int type, int rule, in UrccRequestData[] data);

    void uahResCtlRequestBypass(in UrccRequestParcel request);
}
