#pragma once
/// This all wraps https://gtk-rs.org/gtk-rs-core/stable/latest/docs/glib/struct.MainContext.html#method.channel

#include <inttypes.h>

#include "input-method-unstable-v2-client-protocol.h"
#include "virtual-keyboard-unstable-v1-client-protocol.h"

#include "eek/eek-types.h"
#include "dbus.h"


struct receiver;

/// Wrapped<event_loop::driver::Threaded>
struct squeek_state_manager;

struct submission;

struct rsobjects {
    struct receiver *receiver;
    struct squeek_state_manager *state_manager;
    struct submission *submission;
};

void register_ui_loop_handler(struct receiver *receiver, ServerContextService *ui, DBusHandler *dbus_handler);

struct rsobjects squeek_rsobjects_new(struct zwp_input_method_v2 *im, struct zwp_virtual_keyboard_v1 *vk);

void squeek_state_send_force_visible(struct squeek_state_manager *state);
void squeek_state_send_force_hidden(struct squeek_state_manager *state);

void squeek_state_send_keyboard_present(struct squeek_state_manager *state, uint32_t keyboard_present);
