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

#include <gtk/gtk.h>
#include <glib/gi18n.h>

#include "server-context-service.h"

enum {
    PROP_0,
    PROP_ENABLED,
    PROP_LAST
};

struct _ServerContextService {
    GObject parent;
    struct squeek_state_manager *state_manager; // shared reference
};

G_DEFINE_TYPE(ServerContextService, server_context_service, G_TYPE_OBJECT);

static void
/// Height is in scaled units.
server_context_service_set_property (GObject      *object,
                                     guint         prop_id,
                                     const GValue *value,
                                     GParamSpec   *pspec)
{
    ServerContextService *self = SERVER_CONTEXT_SERVICE(object);

    switch (prop_id) {
    case PROP_ENABLED:
        squeek_state_send_keyboard_present(self->state_manager, !g_value_get_boolean (value));
        break;
    default:
        G_OBJECT_WARN_INVALID_PROPERTY_ID (object, prop_id, pspec);
        break;
    }
}

static void
server_context_service_get_property (GObject    *object,
                                       guint       prop_id,
                                       GValue     *value,
                                       GParamSpec *pspec)
{
    switch (prop_id) {
    default:
        G_OBJECT_WARN_INVALID_PROPERTY_ID (object, prop_id, pspec);
        break;
    }
}

static void
server_context_service_class_init (ServerContextServiceClass *klass)
{
    GObjectClass *gobject_class = G_OBJECT_CLASS (klass);
    GParamSpec *pspec;

    gobject_class->set_property = server_context_service_set_property;
    gobject_class->get_property = server_context_service_get_property;

    /**
     * ServerContextServie:keyboard:
     *
     * Does the user want the keyboard to show up automatically?
     */
    pspec =
        g_param_spec_boolean ("enabled",
                              "Enabled",
                              "Whether the keyboard is enabled",
                              TRUE,
                              G_PARAM_READWRITE | G_PARAM_STATIC_STRINGS);
    g_object_class_install_property (gobject_class,
                                     PROP_ENABLED,
                                     pspec);
}

static void
server_context_service_init (ServerContextService *self) {}


ServerContextService *
server_context_service_new (struct squeek_state_manager *state_manager)
{
    ServerContextService *holder = g_object_new (SERVER_TYPE_CONTEXT_SERVICE, NULL);
    holder->state_manager = state_manager;

    const char *schema_name = "org.gnome.desktop.a11y.applications";
    GSettingsSchemaSource *ssrc = g_settings_schema_source_get_default();
    g_autoptr(GSettingsSchema) schema = NULL;

    if (!ssrc) {
        g_warning("No gsettings schemas installed.");
        return NULL;
    }
    schema = g_settings_schema_source_lookup(ssrc, schema_name, TRUE);
    if (schema) {
        g_autoptr(GSettings) settings = g_settings_new (schema_name);
        g_settings_bind (settings, "screen-keyboard-enabled",
                         holder, "enabled", G_SETTINGS_BIND_GET);
    } else {
        g_warning("Gsettings schema %s is not installed on the system. "
                  "Enabling by default.", schema_name);
    }
    return holder;
}
