package vendor.oplus.hardware.urcc;

import vendor.oplus.hardware.urcc.UrccRequestData;

// IUrccCallback — the HAL calls back into the client with resource state samples.
interface IUrccCallback {
    void onCallback(in UrccRequestData[] data);
}
