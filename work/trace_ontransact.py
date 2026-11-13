import os, re, sys
gen = sys.argv[1]
loginc = '#include <android/log.h>\n'
TAG = '"UrccFramework-halservice"'

def L(fmt, *args):
    arglist = "".join(", " + a for a in args)
    return ('  __android_log_print(ANDROID_LOG_INFO, ' + TAG + ', "' + fmt + '"' +
            arglist + ');\n')

p = os.path.join(gen, "vendor/oplus/hardware/urcc/IUrcc.cpp")
s = open(p).read()
if "onTransact code" not in s:
    if loginc not in s:
        s = loginc + s
    key = "_aidl_vendor_oplus_hardware_urcc_IUrcc_onTransact"
    i = s.find(key)
    b = s.find("{", s.find(")", i))
    s = s[:b+1] + L("onTransact code=%d", "(int)_aidl_code") + s[b+1:]
    s = s.replace(
        "_aidl_ret_status = ::ndk::AParcel_readData(_aidl_in, &in_request);",
        "_aidl_ret_status = ::ndk::AParcel_readData(_aidl_in, &in_request);\n" +
        L("readData status=%d", "(int)_aidl_ret_status"))
    open(p, "w").write(s)
    print("trace: instrumented", p)
else:
    print("trace: IUrcc.cpp already instrumented")

p2 = os.path.join(gen, "vendor/oplus/hardware/urcc/UrccRequestParcel.cpp")
s2 = open(p2).read()
if "field read" not in s2:
    if loginc not in s2:
        s2 = loginc + s2
    def repl(m):
        field = m.group(1)
        return (m.group(0) + "\n" +
                L("field read " + field + " status=%d", "(int)_aidl_ret_status"))
    s2 = re.sub(r"_aidl_ret_status = ::ndk::AParcel_readData\(_aidl_parcel, &([A-Za-z_]+)\);", repl, s2)
    open(p2, "w").write(s2)
    print("trace: instrumented", p2)
else:
    print("trace: UrccRequestParcel.cpp already instrumented")

    # Dump the raw parcel at the start of onTransact so the client's wire
    # format can be compared byte for byte against the generated reader.
    p4 = os.path.join(gen, "vendor/oplus/hardware/urcc/IUrcc.cpp")
    s4 = open(p4).read()
    if "raw parcel" not in s4:
        key = "_aidl_vendor_oplus_hardware_urcc_IUrcc_onTransact"
        i4 = s4.find(key)
        b4 = s4.find("{", s4.find(")", i4))
        dump = """{
      int32_t _sz = AParcel_getDataSize(_aidl_in);
      int32_t _pos = AParcel_getDataPosition(_aidl_in);
      __android_log_print(ANDROID_LOG_INFO, "UrccFramework-halservice",
          "raw parcel size=%d pos=%d", (int)_sz, (int)_pos);
      for (int32_t _k = 0; _k < 40; ++_k) {
        int32_t _v = 0;
        if (AParcel_readInt32(_aidl_in, &_v) != STATUS_OK) break;
        __android_log_print(ANDROID_LOG_INFO, "UrccFramework-halservice",
            "raw[%d]=0x%x", (int)_k, (unsigned)_v);
      }
      AParcel_setDataPosition(_aidl_in, _pos);
    }
"""
        s4 = s4[:b4+1] + dump + s4[b4+1:]
        open(p4, "w").write(s4)
        print("trace: raw parcel dump installed")
