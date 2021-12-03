/*
 * Copyright (C) 2010-2011 Daiki Ueno <ueno@unixuser.org>
 * Copyright (C) 2010-2011 Red Hat, Inc.
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

#include "config.h"

#include "dbus.h"
#include "main.h"

#include <inttypes.h>
#include <stdio.h>
#include <gio/gio.h>

void
dbus_handler_destroy(DBusHandler *service)
{
    g_free (service->object_path);

    if (service->connection) {
        if (service->registration_id > 0) {
            g_dbus_connection_unregister_object (service->connection,
                                                 service->registration_id);
            service->registration_id = 0;
        }

        g_object_unref (service->connection);
        service->connection = NULL;
    }

    if (service->introspection_data) {
        g_dbus_node_info_unref (service->introspection_data);
        service->introspection_data = NULL;
    }

    free(service);
}

static gboolean
handle_set_visible(SmPuriOSK0 *object, GDBusMethodInvocation *invocation,
                   gboolean arg_visible, gpointer user_data) {
    DBusHandler *service = user_data;

    if (arg_visible) {
        squeek_state_send_force_visible (service->state_manager);
    } else {
        squeek_state_send_force_hidden(service->state_manager);
    }

    sm_puri_osk0_complete_set_visible(object, invocation);
    return TRUE;
}

DBusHandler *
dbus_handler_new (GDBusConnection *connection,
                      const gchar     *object_path,
                  struct squeek_state_manager *state_manager)
{
    DBusHandler *self = calloc(1, sizeof(DBusHandler));
    self->object_path = g_strdup(object_path);
    self->connection = connection;
    self->state_manager = state_manager;

    self->dbus_interface = sm_puri_osk0_skeleton_new();
    g_signal_connect(self->dbus_interface, "handle-set-visible",
                     G_CALLBACK(handle_set_visible), self);

    if (self->connection && self->object_path) {
        GError *error = NULL;

        if (!g_dbus_interface_skeleton_export(G_DBUS_INTERFACE_SKELETON(self->dbus_interface),
                                              self->connection,
                                              self->object_path,
                                              &error)) {
            g_warning("Error registering dbus object: %s\n", error->message);
            g_clear_error(&error);
            // TODO: return an error
        }
    }
    return self;
}

// Exported to Rust
void dbus_handler_set_visible(DBusHandler *service,
                       uint8_t visible)
{
    sm_puri_osk0_set_visible(service->dbus_interface, visible);
}
