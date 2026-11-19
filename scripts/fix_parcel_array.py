"""Make the regenerated AIDL transport use the client-side wire format for
arrays of parcelables.

The AIDL backend library used by clients
(vendor.oplus.hardware.urcc-V1-ndk_platform.so) comes from an older backend:
its std::vector<UrccRequestData> elements are written straight through
UrccRequestData::writeToParcel, with no per element null marker. The current
backend routes the same vector through AParcel_writeParcelable, which
prefixes every element with an int32 non-null marker.

The two formats differ on the wire, so a service built with the current
backend cannot parse a request from those clients: the read of the first
array field fails and the whole request is rejected.

This script rewrites the generated sources so that a vector of parcelables
is written and read as
    int32 count
    element[0]::writeToParcel
    element[1]::writeToParcel
which is the format the clients expect.
"""

import os
import sys

gen = sys.argv[1]

READ_OLD = ("_aidl_ret_status = ::ndk::AParcel_readData(_aidl_parcel, "
            "&mUrccRequestData);")
READ_NEW = """{
// Shipped wire format: count, then the elements without a null marker.
int32_t _aidl_count = 0;
_aidl_ret_status = AParcel_readInt32(_aidl_parcel, &_aidl_count);
if (_aidl_ret_status == STATUS_OK && _aidl_count > 0) {
  mUrccRequestData.resize(static_cast<size_t>(_aidl_count));
  for (int32_t _aidl_i = 0; _aidl_i < _aidl_count; ++_aidl_i) {
    _aidl_ret_status =
        mUrccRequestData[static_cast<size_t>(_aidl_i)].readFromParcel(_aidl_parcel);
    if (_aidl_ret_status != STATUS_OK) break;
  }
}
}"""

WRITE_OLD = ("_aidl_ret_status = ::ndk::AParcel_writeData(_aidl_parcel, "
             "&mUrccRequestData);")
WRITE_NEW = """{
// Shipped wire format: count, then the elements without a null marker.
int32_t _aidl_count = static_cast<int32_t>(mUrccRequestData.size());
_aidl_ret_status = AParcel_writeInt32(_aidl_parcel, _aidl_count);
for (size_t _aidl_i = 0; _aidl_ret_status == STATUS_OK &&
                         _aidl_i < mUrccRequestData.size(); ++_aidl_i) {
  _aidl_ret_status = mUrccRequestData[_aidl_i].writeToParcel(_aidl_parcel);
}
}"""

changed = []

parcel = os.path.join(gen, "vendor/oplus/hardware/urcc/UrccRequestParcel.cpp")
if os.path.exists(parcel):
    s = open(parcel).read()
    if "Shipped wire format" not in s:
        if READ_OLD in s:
            s = s.replace(READ_OLD, READ_NEW)
        if WRITE_OLD in s:
            s = s.replace(WRITE_OLD, WRITE_NEW)
        open(parcel, "w").write(s)
        changed.append(parcel)

# The return value of urccResStateRequest is a vector of parcelables as
# well, and the clients read it with the marker-less format.
iface = os.path.join(gen, "vendor/oplus/hardware/urcc/IUrcc.cpp")
if os.path.exists(iface):
    s = open(iface).read()
    marker = "Shipped wire format (return value)"
    if marker not in s:
        old = ("_aidl_ret_status = ::ndk::AParcel_writeData(_aidl_out.get(), "
               "_aidl_return);")
        new = """{
  // Shipped wire format (return value): count, then the elements.
  int32_t _aidl_count = static_cast<int32_t>(_aidl_return.size());
  _aidl_ret_status = AParcel_writeInt32(_aidl_out.get(), _aidl_count);
  for (size_t _aidl_i = 0; _aidl_ret_status == STATUS_OK &&
                           _aidl_i < _aidl_return.size(); ++_aidl_i) {
    _aidl_ret_status = _aidl_return[_aidl_i].writeToParcel(_aidl_out.get());
  }
}"""
        # only the vector<UrccRequestData> return uses this form
        if old in s and "std::vector<::aidl::vendor::oplus::hardware::urcc::UrccRequestData>" in s:
            idx = s.find("std::vector<::aidl::vendor::oplus::hardware::urcc::UrccRequestData>* _aidl_return")
            if idx >= 0:
                j = s.find(old, idx)
                if j >= 0:
                    s = s[:j] + new + s[j+len(old):]
                    open(iface, "w").write(s)
                    changed.append(iface)
for c in changed:
    print("parcel-array patch:", c)
if not changed:
    print("parcel-array patch: nothing to do")
