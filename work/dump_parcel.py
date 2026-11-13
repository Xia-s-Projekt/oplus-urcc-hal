"""Dump the raw request parcel of every transaction into the log.

The generated onTransact is the only place where the client's wire bytes are
still intact, so the dump has to happen before any parcelable is parsed.
"""

import os
import sys

gen = sys.argv[1]
p = os.path.join(gen, "vendor/oplus/hardware/urcc/IUrcc.cpp")
s = open(p).read()
marker = "URCC_RAW_DUMP"
if marker in s:
    print("dump: already installed")
    sys.exit(0)

key = "_aidl_vendor_oplus_hardware_urcc_IUrcc_onTransact"
i = s.find(key)
if i < 0:
    print("dump: onTransact not found")
    sys.exit(1)
b = s.find("{", s.find(")", i))
dump = """{
  // URCC_RAW_DUMP
  int32_t _sz = AParcel_getDataSize(_aidl_in);
  int32_t _pos = AParcel_getDataPosition(_aidl_in);
  __android_log_print(ANDROID_LOG_INFO, "UrccFramework-halservice",
      "raw parcel code=%d size=%d pos=%d", (int)_aidl_code, (int)_sz, (int)_pos);
  for (int32_t _k = 0; _k < 48; ++_k) {
    int32_t _v = 0;
    if (AParcel_readInt32(_aidl_in, &_v) != STATUS_OK) break;
    __android_log_print(ANDROID_LOG_INFO, "UrccFramework-halservice",
        "raw[%d]=0x%x", (int)_k, (unsigned)_v);
  }
  AParcel_setDataPosition(_aidl_in, _pos);
}
"""
s = s[:b+1] + dump + s[b+1:]
if "#include <android/log.h>" not in s:
    s = "#include <android/log.h>\n" + s
open(p, "w").write(s)
print("dump: installed")
