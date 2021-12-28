/* 
 * Copyright (C) 2010-2011 Daiki Ueno <ueno@unixuser.org>
 * Copyright (C) 2010-2011 Red Hat, Inc.
 * Copyright (C) 2019-2020 Purism, SPC
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
#ifndef DBUS_H_
#define DBUS_H_ 1

#include "sm.puri.OSK0.h"

// From main.h
struct squeek_state_manager;

G_BEGIN_DECLS

#define DBUS_SERVICE_PATH "/sm/puri/OSK0"
#define DBUS_SERVICE_INTERFACE "sm.puri.OSK0"

/// Two jobs: accept events, forwarding them to the visibility manager,
/// and get updated from inside to show internal state.
/// Updates are handled in the same loop as the UI.
/// See main.rs
typedef struct _DBusHandler
{
    GDBusConnection *connection;
    SmPuriOSK0 *dbus_interface;
    GDBusNodeInfo *introspection_data;
    guint registration_id;
    char *object_path;

    /// Forward incoming events there
    struct squeek_state_manager *state_manager; // shared reference
} DBusHandler;

DBusHandler * dbus_handler_new      (GDBusConnection *connection,
                                             const gchar     *object_path,
                                     struct squeek_state_manager *state_manager);

void dbus_handler_destroy(DBusHandler*);
G_END_DECLS
#endif  /* DBUS_H_ */
