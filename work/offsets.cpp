// Print the member offsets of the generated UrccRequestParcel / UrccRequestData.
#include <aidl/vendor/oplus/hardware/urcc/UrccRequestParcel.h>
#include <aidl/vendor/oplus/hardware/urcc/UrccRequestData.h>

#include <cstdio>
#include <cstddef>

using namespace aidl::vendor::oplus::hardware::urcc;

int main() {
    printf("UrccRequestParcel size=%zu\n", sizeof(UrccRequestParcel));
    printf("  type             %zu\n", offsetof(UrccRequestParcel, type));
    printf("  mUrccRequestData %zu\n", offsetof(UrccRequestParcel, mUrccRequestData));
    printf("  screenoff_support%zu\n", offsetof(UrccRequestParcel, screenoff_support));
    printf("  prio             %zu\n", offsetof(UrccRequestParcel, prio));
    printf("  callingPID       %zu\n", offsetof(UrccRequestParcel, callingPID));
    printf("  callingUID       %zu\n", offsetof(UrccRequestParcel, callingUID));
    printf("  duration         %zu\n", offsetof(UrccRequestParcel, duration));
    printf("  mParams          %zu\n", offsetof(UrccRequestParcel, mParams));
    printf("  handle           %zu\n", offsetof(UrccRequestParcel, handle));
    printf("  urccCallback     %zu\n", offsetof(UrccRequestParcel, urccCallback));
    printf("  pkgName          %zu\n", offsetof(UrccRequestParcel, pkgName));
    printf("  identity         %zu\n", offsetof(UrccRequestParcel, identity));
    printf("  otherData        %zu\n", offsetof(UrccRequestParcel, otherData));
    printf("UrccRequestData size=%zu\n", sizeof(UrccRequestData));
    printf("  res_Id    %zu\n", offsetof(UrccRequestData, res_Id));
    printf("  event_Id  %zu\n", offsetof(UrccRequestData, event_Id));
    printf("  res_Value %zu\n", offsetof(UrccRequestData, res_Value));
    printf("  res_Path  %zu\n", offsetof(UrccRequestData, res_Path));
    printf("  desc      %zu\n", offsetof(UrccRequestData, desc));
    return 0;
}
