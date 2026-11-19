"""Make the regenerated AIDL transport tolerate null strings and null arrays.

The framework's Java stub writes a null String as a length of -1. The
platform helper that the generated code uses for a plain std::string rejects
that length, so the whole request fails to parse and the transaction is
dropped before it reaches the implementation:

    field read mUrccRequestData status=-2147483640   (STATUS_UNEXPECTED_NULL)

The AIDL backend used by clients read the same data with an allocator that
accepts a non-positive length and yields an empty string. This script rewrites
the generated sources to use such an allocator, so the transport parses the
framework's requests the same way.
"""

import os
import re
import sys

gen = sys.argv[1]

HELPER = """namespace {
// The framework writes a null String as a length of -1. The platform
// allocator for std::string rejects that, so it is replaced here by one that
// maps a non-positive length to an empty string, like the client-side backend.
bool urcc_lenient_string_allocator(void* stringData, int32_t length, char** buffer) {
  std::string* str = static_cast<std::string*>(stringData);
  if (length <= 0) {
    // The platform reports a null string by calling the allocator with a
    // length of -1 and a null buffer pointer, and it requires the allocator
    // to accept it. Returning false here is what makes the platform fail the
    // whole request with STATUS_UNEXPECTED_NULL.
    str->clear();
    if (buffer != nullptr) *buffer = nullptr;
    return true;
  }
  str->resize(static_cast<size_t>(length) - 1);
  if (buffer != nullptr) *buffer = &(*str)[0];
  return true;
}

binder_status_t urcc_read_string(const AParcel* parcel, std::string* str) {
  return AParcel_readString(parcel, static_cast<void*>(str),
                             urcc_lenient_string_allocator);
}

// A null String[] arrives as a length of -1; the platform helper for
// std::vector<std::string> rejects that as well.
binder_status_t urcc_read_string_array(const AParcel* parcel,
                                       std::vector<std::string>* vec) {
  int32_t length = 0;
  binder_status_t status = AParcel_readInt32(parcel, &length);
  if (status != STATUS_OK) return status;
  vec->clear();
  if (length <= 0) return STATUS_OK;
  vec->reserve(static_cast<size_t>(length));
  for (int32_t i = 0; i < length; ++i) {
    std::string element;
    status = urcc_read_string(parcel, &element);
    if (status != STATUS_OK) return status;
    vec->push_back(std::move(element));
  }
  return STATUS_OK;
}

// A null byte[] arrives as a length of -1; the platform helper for
// std::vector<uint8_t> rejects that as well.
binder_status_t urcc_read_byte_array(const AParcel* parcel,
                                     std::vector<uint8_t>* vec) {
  int32_t length = 0;
  binder_status_t status = AParcel_readInt32(parcel, &length);
  if (status != STATUS_OK) return status;
  vec->clear();
  if (length <= 0) return STATUS_OK;
  vec->resize(static_cast<size_t>(length));
  for (size_t i = 0; i < vec->size(); ++i) {
    int8_t byte = 0;
    status = AParcel_readByte(parcel, &byte);
    if (status != STATUS_OK) return status;
    (*vec)[i] = static_cast<uint8_t>(byte);
  }
  return STATUS_OK;
}
}  // namespace
"""

STRING_FIELDS = ("res_Value", "res_Path", "desc", "pkgName", "identity")
ARRAY_FIELDS = ("mParams",)


def patch(path, fields, array_fields):
    if not os.path.exists(path):
        return False
    s = open(path).read()
    if "urcc_lenient_string_allocator" in s:
        return False
    changed = False
    for field in fields:
        old = ("_aidl_ret_status = ::ndk::AParcel_readData(_aidl_parcel, &%s);" % field)
        new = ("_aidl_ret_status = urcc_read_string(_aidl_parcel, &%s);" % field)
        if old in s:
            s = s.replace(old, new)
            changed = True
    for field in array_fields:
        old = ("_aidl_ret_status = ::ndk::AParcel_readData(_aidl_parcel, &%s);" % field)
        new = ("_aidl_ret_status = urcc_read_string_array(_aidl_parcel, &%s);" % field)
        if old in s:
            s = s.replace(old, new)
            changed = True
    if not changed:
        return False
    anchor = "namespace aidl {"
    if anchor in s:
        s = s.replace(anchor, HELPER + chr(10) + anchor, 1)
    if "#include <string>" not in s:
        s = "#include <string>\n#include <vector>\n" + s
    open(path, "w").write(s)
    return True



def patch_bytes(path):
    if not os.path.exists(path):
        return False
    s = open(path).read()
    if "urcc_read_byte_array(_aidl_parcel" in s:
        return False
    old = "_aidl_ret_status = ::ndk::AParcel_readData(_aidl_parcel, &otherData);"
    new = "_aidl_ret_status = urcc_read_byte_array(_aidl_parcel, &otherData);"
    if old not in s:
        return False
    s = s.replace(old, new)
    open(path, "w").write(s)
    return True


base = os.path.join(gen, "vendor/oplus/hardware/urcc")
touched = []
if patch(os.path.join(base, "UrccRequestData.cpp"), STRING_FIELDS, ()):
    touched.append("UrccRequestData.cpp")
if patch(os.path.join(base, "UrccRequestParcel.cpp"), STRING_FIELDS, ARRAY_FIELDS):
    touched.append("UrccRequestParcel.cpp")
if patch_bytes(os.path.join(base, "UrccRequestParcel.cpp")):
    touched.append("UrccRequestParcel.cpp(otherData)")
print("lenient strings: patched", touched if touched else "nothing")
