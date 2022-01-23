#ifndef WAYLAND_H
#define WAYLAND_H

#include <gmodule.h>

#include "wlr-layer-shell-unstable-v1-client-protocol.h"
#include "virtual-keyboard-unstable-v1-client-protocol.h"
#include "input-method-unstable-v2-client-protocol.h"

#include "outputs.h"

struct squeek_wayland {
    // globals
    struct zwlr_layer_shell_v1 *layer_shell;
    struct zwp_virtual_keyboard_manager_v1 *virtual_keyboard_manager;
    struct zwp_input_method_manager_v2 *input_method_manager;
    struct squeek_outputs *outputs;
    struct wl_seat *seat;
    // objects
    struct zwp_input_method_v2 *input_method;
    struct zwp_virtual_keyboard_v1 *virtual_keyboard;
};


extern struct squeek_wayland *squeek_wayland;

#endif // WAYLAND_H
