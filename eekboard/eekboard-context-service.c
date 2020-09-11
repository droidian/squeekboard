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

#include <stdio.h>

#include <gio/gio.h>

#include "wayland.h"

#include "eek/eek-keyboard.h"
#include "src/server-context-service.h"

#include "eekboard/eekboard-context-service.h"

enum {
    PROP_0, // Magic: without this, keyboard is not useable in g_object_notify
    PROP_KEYBOARD,
    PROP_LAST
};

enum {
    DESTROYED,
    LAST_SIGNAL
};

static guint signals[LAST_SIGNAL] = { 0, };

struct _EekboardContextServicePrivate {
    LevelKeyboard *keyboard; // currently used keyboard
    GSettings *settings; // Owned reference

    // Maybe TODO: it's used only for fetching layout type.
    // Maybe let UI push the type to this structure?
    ServerContextService *ui; // unowned reference
    /// Needed for keymap changes after keyboard updates
    struct submission *submission; // unowned
};

G_DEFINE_TYPE_WITH_PRIVATE (EekboardContextService, eekboard_context_service, G_TYPE_OBJECT);

static void
eekboard_context_service_set_property (GObject      *object,
                                       guint         prop_id,
                                       const GValue *value,
                                       GParamSpec   *pspec)
{
    (void)value;
    switch (prop_id) {
    default:
        G_OBJECT_WARN_INVALID_PROPERTY_ID (object, prop_id, pspec);
        break;
    }
}

static void
eekboard_context_service_get_property (GObject    *object,
                                       guint       prop_id,
                                       GValue     *value,
                                       GParamSpec *pspec)
{
    EekboardContextService *context = EEKBOARD_CONTEXT_SERVICE(object);

    switch (prop_id) {
    case PROP_KEYBOARD:
        g_value_set_object (value, context->priv->keyboard);
        break;
    default:
        G_OBJECT_WARN_INVALID_PROPERTY_ID (object, prop_id, pspec);
        break;
    }
}

static void
eekboard_context_service_dispose (GObject *object)
{
    G_OBJECT_CLASS (eekboard_context_service_parent_class)->
        dispose (object);
}

static void
settings_get_layout(GSettings *settings, char **type, char **layout)
{
    if (!settings) {
        return;
    }
    GVariant *inputs = g_settings_get_value(settings, "sources");
    if (g_variant_n_children(inputs) == 0) {
        g_warning("No system layout present");
        *type = NULL;
        *layout = NULL;
    } else {
        // current layout is always first
        g_variant_get_child(inputs, 0, "(ss)", type, layout);
    }
    g_variant_unref(inputs);
}

void
eekboard_context_service_use_layout(EekboardContextService *context, struct squeek_layout_state *state, uint32_t timestamp) {
    gchar *layout_name = state->overlay_name;

    if (layout_name == NULL) {
        layout_name = state->layout_name;

        switch (state->purpose) {
        case ZWP_TEXT_INPUT_V3_CONTENT_PURPOSE_NUMBER:
        case ZWP_TEXT_INPUT_V3_CONTENT_PURPOSE_PHONE:
            layout_name = "number";
            break;
        case ZWP_TEXT_INPUT_V3_CONTENT_PURPOSE_TERMINAL:
            layout_name = "terminal";
            break;
        default:
            ;
        }

        if (layout_name == NULL) {
            layout_name = "us";
        }
    }

    // generic part follows
    struct squeek_layout *layout = squeek_load_layout(layout_name, state->arrangement);
    LevelKeyboard *keyboard = level_keyboard_new(layout);
    // set as current
    LevelKeyboard *previous_keyboard = context->priv->keyboard;
    context->priv->keyboard = keyboard;
    // Update the keymap if necessary.
    // TODO: Update submission on change event
    if (context->priv->submission) {
        submission_set_keyboard(context->priv->submission, keyboard, timestamp);
    }

    // Update UI
    g_object_notify (G_OBJECT(context), "keyboard");

    // replacing the keyboard above will cause the previous keyboard to get destroyed from the UI side (eek_gtk_keyboard_dispose)
    if (previous_keyboard) {
        level_keyboard_free(previous_keyboard);
    }
}

static void eekboard_context_service_update_settings_layout(EekboardContextService *context) {
    g_autofree gchar *keyboard_layout = NULL;
    g_autofree gchar *keyboard_type = NULL;
    settings_get_layout(context->priv->settings,
                        &keyboard_type, &keyboard_layout);

    if (g_strcmp0(context->layout->layout_name, keyboard_layout) != 0 || context->layout->overlay_name) {
        g_free(context->layout->overlay_name);
        context->layout->overlay_name = NULL;
        if (keyboard_layout) {
            g_free(context->layout->layout_name);
            context->layout->layout_name = g_strdup(keyboard_layout);
        }
        // This must actually update the UI.
        uint32_t time = gdk_event_get_time(NULL);
        eekboard_context_service_use_layout(context, context->layout, time);
    }
}

static gboolean
settings_handle_layout_changed(GSettings *s,
                               gpointer keys, gint n_keys,
                               gpointer user_data) {
    (void)s;
    (void)keys;
    (void)n_keys;
    EekboardContextService *context = user_data;
    eekboard_context_service_update_settings_layout(context);
    return TRUE;
}

static void
eekboard_context_service_constructed (GObject *object)
{
    (void)object;
}

static void
eekboard_context_service_class_init (EekboardContextServiceClass *klass)
{
    GObjectClass *gobject_class = G_OBJECT_CLASS (klass);
    GParamSpec *pspec;

    gobject_class->constructed = eekboard_context_service_constructed;
    gobject_class->set_property = eekboard_context_service_set_property;
    gobject_class->get_property = eekboard_context_service_get_property;
    gobject_class->dispose = eekboard_context_service_dispose;
    /**
     * EekboardContextService::destroyed:
     * @context: an #EekboardContextService
     *
     * Emitted when @context is destroyed.
     */
    signals[DESTROYED] =
        g_signal_new (I_("destroyed"),
                      G_TYPE_FROM_CLASS(gobject_class),
                      G_SIGNAL_RUN_LAST,
                      G_STRUCT_OFFSET(EekboardContextServiceClass, destroyed),
                      NULL,
                      NULL,
                      g_cclosure_marshal_VOID__VOID,
                      G_TYPE_NONE,
                      0);

    /**
     * EekboardContextService:keyboard:
     *
     * An #EekKeyboard currently active in this context.
     */
    pspec = g_param_spec_pointer("keyboard",
                                 "Keyboard",
                                 "Keyboard",
                                 G_PARAM_READABLE);
    g_object_class_install_property (gobject_class,
                                     PROP_KEYBOARD,
                                     pspec);
}

static void
eekboard_context_service_init (EekboardContextService *self)
{
    self->priv = eekboard_context_service_get_instance_private (self);

    const char *schema_name = "org.gnome.desktop.input-sources";
    GSettingsSchemaSource *ssrc = g_settings_schema_source_get_default();
    g_autoptr(GSettingsSchema) schema = NULL;

    if (!ssrc) {
        g_warning("No gsettings schemas installed. Layout switching unavailable.");
        return;
    }

    schema = g_settings_schema_source_lookup(ssrc, schema_name, TRUE);
    if (schema) {
        // Not referencing the found schema directly,
        // because it's not clear how...
        self->priv->settings = g_settings_new (schema_name);
        gulong conn_id = g_signal_connect(self->priv->settings, "change-event",
                                          G_CALLBACK(settings_handle_layout_changed),
                                          self);
        if (conn_id == 0) {
            g_warning ("Could not connect to gsettings updates, "
                       "automatic layout changing unavailable");
        }
    } else {
        g_warning("Gsettings schema %s is not installed on the system. "
                  "Layout switching unavailable", schema_name);
    }
}

/**
 * eekboard_context_service_destroy:
 * @context: an #EekboardContextService
 *
 * Destroy @context.
 */
void
eekboard_context_service_destroy (EekboardContextService *context)
{
    g_return_if_fail (EEKBOARD_IS_CONTEXT_SERVICE(context));
    g_signal_emit (context, signals[DESTROYED], 0);
}

/**
 * eekboard_context_service_get_keyboard:
 * @context: an #EekboardContextService
 *
 * Get keyboard currently active in @context.
 * Returns: (transfer none): an #EekKeyboard
 */
LevelKeyboard *
eekboard_context_service_get_keyboard (EekboardContextService *context)
{
    return context->priv->keyboard;
}

void eekboard_context_service_set_hint_purpose(EekboardContextService *context,
                                               uint32_t hint, uint32_t purpose)
{
    if (context->layout->hint != hint || context->layout->purpose != purpose) {
        context->layout->hint = hint;
        context->layout->purpose = purpose;
        uint32_t time = gdk_event_get_time(NULL);
        eekboard_context_service_use_layout(context, context->layout, time);
    }
}

void
eekboard_context_service_set_overlay(EekboardContextService *context, const char* name) {
    if (g_strcmp0(context->layout->overlay_name, name)) {
        g_free(context->layout->overlay_name);
        context->layout->overlay_name = g_strdup(name);
        uint32_t time = gdk_event_get_time(NULL);
        eekboard_context_service_use_layout(context, context->layout, time);
    }
}

const char*
eekboard_context_service_get_overlay(EekboardContextService *context) {
    return context->layout->overlay_name;
}

EekboardContextService *eekboard_context_service_new(struct squeek_layout_state *state)
{
    EekboardContextService *context = g_object_new (EEKBOARD_TYPE_CONTEXT_SERVICE, NULL);
    context->layout = state;
    eekboard_context_service_update_settings_layout(context);
    uint32_t time = gdk_event_get_time(NULL);
    eekboard_context_service_use_layout(context, context->layout, time);
    return context;
}

void eekboard_context_service_set_submission(EekboardContextService *context, struct submission *submission) {
    context->priv->submission = submission;
    if (context->priv->submission) {
        uint32_t time = gdk_event_get_time(NULL);
        submission_set_keyboard(context->priv->submission, context->priv->keyboard, time);
    }
}

void eekboard_context_service_set_ui(EekboardContextService *context, ServerContextService *ui) {
    context->priv->ui = ui;
}
