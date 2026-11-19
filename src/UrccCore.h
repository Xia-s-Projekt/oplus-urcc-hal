// Native URCC resource core.
//
// The OnePlus resource ids come in two encodings:
//
//   library form  0x01010ba0  (mode in bits 24..27, module in bits 16..23)
//   framework form 0x1010ba0  (the same value with the mode nibble moved to
//                                bits 28..31)
//
// normalize_id() folds both onto the library form, so a request from either
// side finds the same resource.
#ifndef OPLUS_URCC_URCCCORE_H
#define OPLUS_URCC_URCCCORE_H

#include <cstddef>
#include <cstdint>
#include <string>

namespace aidl::vendor::oplus::hardware::urcc::impl::oplus {

// How a resource value is served.
enum class Kind {
    kRead,       // the value is read from a file
    kWrite,      // a control request writes the file
    kReadWrite,  // both
    kDerived,    // the value is computed from other sources
    kApplied,    // no node: a control request performs the equivalent action
};

struct UrccResource {
    uint32_t id;
    const char* name;
    int module;        // 1 cpu, 2 gpu, 3 mem, 5 thermal, 6 misc
    Kind kind;
    const char* path;  // set for kRead/kWrite/kReadWrite
    const char* action;  // set for kDerived/kApplied
};

class UrccCore {
  public:
    UrccCore();
    ~UrccCore();

    // Current value of a resource, as the string the framework expects.
    // "-1" when the resource is unknown or unreadable.
    std::string read(uint32_t id);

    // Applies a control request. Returns false and fills error when the
    // resource cannot take the value.
    bool apply(uint32_t id, const std::string& value, std::string* error);

    // Path-addressed requests. The UAH client drives the core by node path
    // (liboplus-uah-client.so: urcc_resctl_request_by_path), so a request
    // that names a node is applied to that node. Only supported nodes are
    // writable, so a request for another node is refused instead of silently
    // doing nothing.
    bool write_node(const std::string& path, const std::string& value);
    std::string read_node(const std::string& path);

    // The resource's OnePlus name, or "" when unknown.
    const char* name_of(uint32_t id);

    // The node the resource is served from, or "" when it has none.
    std::string path_of(uint32_t id);

  private:
    const UrccResource* find(uint32_t id) const;
    std::string derive(const char* action);
    std::string cluster_load(int cluster);

    long long mPrevGpuBusy = 0;
    long long mPrevGpuIdle = 0;
};

}  // namespace aidl::vendor::oplus::hardware::urcc::impl::oplus

#endif  // OPLUS_URCC_URCCCORE_H
