#include "wp_api_os_log.h"

#ifdef __APPLE__
void wp_api_os_log_with_type(os_log_t log, os_log_type_t type, const char *message) {
    os_log_with_type(log, type, "%{public}s", message);
}
#endif
