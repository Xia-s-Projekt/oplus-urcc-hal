"""Read the request parcel manually, with per-step logging.

The generated reader fails inside the array field and the platform helper
hides which step went wrong, so the array is read by hand here: length,
then the per element presence flag, then the element itself.
"""

import os
import sys

gen = sys.argv[1]
p = os.path.join(gen, "vendor/oplus/hardware/urcc/UrccRequestParcel.cpp")
s = open(p).read()
marker = "URCC_MANUAL_ARRAY"
if marker in s:
    print("manual array: already installed")
    sys.exit(0)

OLD = "_aidl_ret_status = ::ndk::AParcel_readData(_aidl_parcel, &mUrccRequestData);"
NEW = """{
    // URCC_MANUAL_ARRAY
    int32_t _len = 0;
    _aidl_ret_status = AParcel_readInt32(_aidl_parcel, &_len);
    __android_log_print(ANDROID_LOG_INFO, "UrccFramework-halservice",
        "array len=%d status=%d", (int)_len, (int)_aidl_ret_status);
    if (_aidl_ret_status == STATUS_OK && _len > 0) {
      mUrccRequestData.resize(static_cast<size_t>(_len));
      for (int32_t _i = 0; _i < _len; ++_i) {
        int32_t _present = 0;
        _aidl_ret_status = AParcel_readInt32(_aidl_parcel, &_present);
        __android_log_print(ANDROID_LOG_INFO, "UrccFramework-halservice",
            "array[%d] present=%d status=%d", (int)_i, (int)_present,
            (int)_aidl_ret_status);
        if (_aidl_ret_status != STATUS_OK) break;
        if (_present == 0) {
          __android_log_print(ANDROID_LOG_INFO, "UrccFramework-halservice",
              "array[%d] is null", (int)_i);
          break;
        }
        _aidl_ret_status =
            mUrccRequestData[static_cast<size_t>(_i)].readFromParcel(_aidl_parcel);
        __android_log_print(ANDROID_LOG_INFO, "UrccFramework-halservice",
            "array[%d] element status=%d", (int)_i, (int)_aidl_ret_status);
        if (_aidl_ret_status != STATUS_OK) break;
      }
    }
  }"""
if OLD not in s:
    print("manual array: pattern not found")
    sys.exit(1)
s = s.replace(OLD, NEW)
if "#include <android/log.h>" not in s:
    s = "#include <android/log.h>\n" + s
open(p, "w").write(s)
print("manual array: installed")
