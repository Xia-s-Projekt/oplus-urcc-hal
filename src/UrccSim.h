// Companion simulator for the URCC HAL.
//
// See UrccSim.cpp for what it computes and where each value comes from.
#ifndef OPLUS_URCC_URCCSIM_H
#define OPLUS_URCC_URCCSIM_H

namespace aidl::vendor::oplus::hardware::urcc::impl::oplus {

class UrccSim {
  public:
    UrccSim();
    ~UrccSim();

    // Starts the refresh thread. Never returns.
    void start();

  private:
    bool mRunning = false;
};

}  // namespace aidl::vendor::oplus::hardware::urcc::impl::oplus

#endif  // OPLUS_URCC_URCCSIM_H
