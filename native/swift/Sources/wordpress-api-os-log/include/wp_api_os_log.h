#ifndef WP_API_OS_LOG_H
#define WP_API_OS_LOG_H

// Called by the Rust logger in `wp_api/src/os_log.rs`. `os_log_with_type` is a macro, so Rust can't
// call it directly. This file is compiled by Xcode as part of the Swift package, which keeps the
// Apple SDK out of the Rust build. On other platforms the Rust library doesn't log through os_log,
// so this is empty.
#ifdef __APPLE__
#include <os/log.h>

void wp_api_os_log_with_type(os_log_t log, os_log_type_t type, const char *message);
#endif

#endif
