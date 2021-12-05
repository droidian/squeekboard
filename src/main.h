#pragma once
/// This all wraps https://gtk-rs.org/gtk-rs-core/stable/latest/docs/glib/struct.MainContext.html#method.channel

#include "eek/eek-types.h"
#include "dbus.h"

struct receiver;
struct sender;

struct channel {
    struct sender *sender;
    struct receiver *receiver;
};

/// Creates a channel with one end inside the glib main loop
struct channel main_loop_channel_new(void);
void register_ui_loop_handler(struct receiver *receiver, ServerContextService *ui, DBusHandler *dbus_handler);
