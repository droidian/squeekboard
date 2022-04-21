#include "input-method-unstable-v2-client-protocol.h"
#include "virtual-keyboard-unstable-v1-client-protocol.h"

#include "submission.h"

struct imservice;

void imservice_handle_input_method_activate(void *data, struct zwp_input_method_v2 *input_method);
void imservice_handle_input_method_deactivate(void *data, struct zwp_input_method_v2 *input_method);
void imservice_handle_surrounding_text(void *data, struct zwp_input_method_v2 *input_method,
                                       const char *text, uint32_t cursor, uint32_t anchor);
void imservice_handle_done(void *data, struct zwp_input_method_v2 *input_method);
void imservice_handle_content_type(void *data, struct zwp_input_method_v2 *input_method, uint32_t hint, uint32_t purpose);
void imservice_handle_text_change_cause(void *data, struct zwp_input_method_v2 *input_method, uint32_t cause);
void imservice_handle_unavailable(void *data, struct zwp_input_method_v2 *input_method);

static const struct zwp_input_method_v2_listener input_method_listener = {
    .activate = imservice_handle_input_method_activate,
    .deactivate = imservice_handle_input_method_deactivate,
    .surrounding_text = imservice_handle_surrounding_text,
    .text_change_cause = imservice_handle_text_change_cause,
    .content_type = imservice_handle_content_type,
    .done = imservice_handle_done,
    .unavailable = imservice_handle_unavailable,
};

/// Un-inlined
struct zwp_input_method_v2 *imservice_manager_get_input_method(struct zwp_input_method_manager_v2 *manager,
                                   struct wl_seat *seat) {
    return zwp_input_method_manager_v2_get_input_method(manager, seat);
}

/// Un-inlined to let Rust link to it
void imservice_connect_listeners(struct zwp_input_method_v2 *im, struct imservice* imservice) {
    zwp_input_method_v2_add_listener(im, &input_method_listener, imservice);
}

void
eek_input_method_commit_string(struct zwp_input_method_v2 *zwp_input_method_v2, const char *text)
{
    zwp_input_method_v2_commit_string(zwp_input_method_v2, text);
}

void
eek_input_method_delete_surrounding_text(struct zwp_input_method_v2 *zwp_input_method_v2, uint32_t before_length, uint32_t after_length) {
    zwp_input_method_v2_delete_surrounding_text(zwp_input_method_v2, before_length, after_length);
};

void
eek_input_method_commit(struct zwp_input_method_v2 *zwp_input_method_v2, uint32_t serial)
{
    zwp_input_method_v2_commit(zwp_input_method_v2, serial);
}

/// Declared explicitly because _destroy is inline,
/// making it unavailable in Rust
void imservice_destroy_im(struct zwp_input_method_v2 *im) {
    zwp_input_method_v2_destroy(im);
}
