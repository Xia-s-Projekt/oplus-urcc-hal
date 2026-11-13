// Calls the shipped framework client's own API (liboplus-uah-client.so,
// system_ext copy) against the running service, outside system_server.
//
// The framework log says "urcc HAL service not available", which is the
// client's own error string; this reproduces the client's lookup path.
#include <dlfcn.h>

#include <cstdio>
#include <cstdlib>
#include <cstring>

extern "C" int uahInit();

int main() {
    // Optionally switch to the framework's SELinux domain so the lookup runs
    // under the same policy context as system_server.
    const char* ctx = getenv("UAH_SELINUX_CTX");
    if (ctx != nullptr && ctx[0] != '\0') {
        FILE* f = fopen("/proc/self/attr/current", "w");
        if (f == nullptr) {
            std::printf("cannot open attr/current\n");
        } else {
            fwrite(ctx, 1, strlen(ctx), f);
            fclose(f);
            std::printf("switched to %s\n", ctx);
        }
    }
    const char* path = "/system_ext/lib64/liboplus-uah-client.so";
    void* lib = dlopen(path, RTLD_NOW);
    if (lib == nullptr) {
        std::printf("dlopen failed: %s\n", dlerror());
        return 1;
    }
    auto init = reinterpret_cast<int (*)()>(dlsym(lib, "uahInit"));
    if (init == nullptr) {
        std::printf("dlsym uahInit failed: %s\n", dlerror());
        return 1;
    }
    int rc = init();
    std::printf("uahInit() -> %d\n", rc);
    {
        FILE* f = fopen("/proc/self/attr/current", "r");
        char buf[128] = {0};
        if (f != nullptr) {
            if (fgets(buf, sizeof(buf), f) == nullptr) { buf[0] = 0; }
            fclose(f);
        }
        std::printf("actual context: %s", buf);
    }
    return rc == 0 ? 0 : 1;
}
