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

#include "eek/eek.h"
#include "eek/eek-gtk-keyboard.h"
#include "eek/layersurface.h"
#include "eekboard/eekboard-context-service.h"
#include "submission.h"
#include "wayland.h"
#include "server-context-service.h"
#include "wayland-client-protocol.h"

enum {
    PROP_0,
    PROP_ENABLED,
    PROP_LAST
};

struct _ServerContextService {
    GObject parent;

    EekboardContextService *state; // unowned
    /// Needed for instantiating the widget
    struct submission *submission; // unowned
    struct squeek_layout_state *layout;
    struct ui_manager *manager; // unowned
    struct squeek_state_manager *state_manager; // shared reference

    PhoshLayerSurface *window;
    GtkWidget *widget; // nullable
    guint last_requested_height;
};

G_DEFINE_TYPE(ServerContextService, server_context_service, G_TYPE_OBJECT);

static void
on_destroy (ServerContextService *self, GtkWidget *widget)
{
    g_return_if_fail (SERVER_IS_CONTEXT_SERVICE (self));

    g_assert (widget == GTK_WIDGET(self->window));

    self->window = NULL;
    self->widget = NULL;

    //eekboard_context_service_destroy (EEKBOARD_CONTEXT_SERVICE (context));
}

static uint32_t
calculate_height(int32_t width, GdkRectangle *geometry)
{
    uint32_t height;
    if (geometry->width > geometry->height) {
        // 1:5 ratio works fine on lanscape mode, and makes sure there's
        // room left for the app window
        height = width / 5;
    } else {
        if (width < 540 && width > 0) {
            height = ((unsigned)width * 7 / 12); // to match 360×210
        } else {
            // Here we switch to wide layout, less height needed
            height = ((unsigned)width * 7 / 22);
        }
    }
    return height;
}

static void
on_surface_configure(ServerContextService *self, PhoshLayerSurface *surface)
{
    GdkDisplay *display = NULL;
    GdkWindow *window = NULL;
    GdkMonitor *monitor = NULL;
    GdkRectangle geometry;
    gint width;
    gint height;

    g_return_if_fail (SERVER_IS_CONTEXT_SERVICE (self));
    g_return_if_fail (PHOSH_IS_LAYER_SURFACE (surface));

    g_object_get(G_OBJECT(surface),
                 "configured-width", &width,
                 "configured-height", &height,
                 NULL);

    // In order to improve height calculation, we need the monitor geometry so
    // we can use different algorithms for portrait and landscape mode.
    // Note: this is a temporary fix until the size manager is complete.
    display = gdk_display_get_default ();
    if (display) {
        window = gtk_widget_get_window (GTK_WIDGET (surface));
    }
    if (window) {
        monitor = gdk_display_get_monitor_at_window (display, window);
    }
    if (monitor) {
        gdk_monitor_get_geometry (monitor, &geometry);
    } else {
        geometry.width = geometry.height = 0;
    }

     // When the geometry event comes after surface.configure,
     // this entire height calculation does nothing.
     // guint desired_height = squeek_uiman_get_perceptual_height(context->manager);
     // Temporarily use old method, until the size manager is complete.
    guint desired_height = calculate_height(width, &geometry);

    guint configured_height = (guint)height;
    // if height was already requested once but a different one was given
    // (for the same set of surrounding properties),
    // then it's probably not reasonable to ask for it again,
    // as it's likely to create pointless loops
    // of request->reject->request_again->...
    if (desired_height != configured_height
            && self->last_requested_height != desired_height) {
        self->last_requested_height = desired_height;
        phosh_layer_surface_set_size(surface, 0,
                                     (gint)desired_height);
        phosh_layer_surface_set_exclusive_zone(surface, (gint)desired_height);
        phosh_layer_surface_wl_surface_commit (surface);
    }
}

static void
make_window (ServerContextService *self, struct wl_output *output)
{
    if (self->window) {
        g_error("Window already present");
    }

    uint32_t height = squeek_uiman_get_perceptual_height(self->manager);

    self->window = g_object_new (
        PHOSH_TYPE_LAYER_SURFACE,
        "layer-shell", squeek_wayland->layer_shell,
        "wl-output", output,
        "height", height,
        "anchor", ZWLR_LAYER_SURFACE_V1_ANCHOR_BOTTOM
                  | ZWLR_LAYER_SURFACE_V1_ANCHOR_LEFT
                  | ZWLR_LAYER_SURFACE_V1_ANCHOR_RIGHT,
        "layer", ZWLR_LAYER_SHELL_V1_LAYER_TOP,
        "kbd-interactivity", FALSE,
        "exclusive-zone", height,
        "namespace", "osk",
        NULL
    );

    g_object_connect (self->window,
                      "swapped-signal::destroy", G_CALLBACK(on_destroy), self,
                      "swapped-signal::configured", G_CALLBACK(on_surface_configure), self,
                      NULL);

    // The properties below are just to make hacking easier.
    // The way we use layer-shell overrides some,
    // and there's no space in the protocol for others.
    // Those may still be useful in the future,
    // or for hacks with regular windows.
    gtk_widget_set_can_focus (GTK_WIDGET(self->window), FALSE);
    g_object_set (G_OBJECT(self->window), "accept_focus", FALSE, NULL);
    gtk_window_set_title (GTK_WINDOW(self->window), "Squeekboard");
    gtk_window_set_icon_name (GTK_WINDOW(self->window), "squeekboard");
    gtk_window_set_keep_above (GTK_WINDOW(self->window), TRUE);
}

static void
destroy_window (ServerContextService *self)
{
    gtk_widget_destroy (GTK_WIDGET (self->window));
    self->window = NULL;
}

static void
make_widget (ServerContextService *self)
{
    if (self->widget) {
        gtk_widget_destroy(self->widget);
        self->widget = NULL;
    }
    self->widget = eek_gtk_keyboard_new (self->state, self->submission, self->layout);

    gtk_widget_set_has_tooltip (self->widget, TRUE);
    gtk_container_add (GTK_CONTAINER(self->window), self->widget);
    gtk_widget_show_all(self->widget);
}

// Called from rust
void
server_context_service_real_show_keyboard (ServerContextService *self, struct wl_output *output)
{
    if (!self->window) {
        make_window (self, output);
    }
    if (!self->widget) {
        make_widget (self);
    }
    gtk_widget_show (GTK_WIDGET(self->window));
}

// Called from rust
void
server_context_service_real_hide_keyboard (ServerContextService *self)
{
    if (self->window) {
	    gtk_widget_hide (GTK_WIDGET(self->window));
    }
}

static void
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
server_context_service_dispose (GObject *object)
{
    ServerContextService *self = SERVER_CONTEXT_SERVICE(object);

    destroy_window (self);
    self->widget = NULL;

    G_OBJECT_CLASS (server_context_service_parent_class)->dispose (object);
}

static void
server_context_service_class_init (ServerContextServiceClass *klass)
{
    GObjectClass *gobject_class = G_OBJECT_CLASS (klass);
    GParamSpec *pspec;

    gobject_class->set_property = server_context_service_set_property;
    gobject_class->get_property = server_context_service_get_property;
    gobject_class->dispose = server_context_service_dispose;

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

static void
init (ServerContextService *self) {
    const char *schema_name = "org.gnome.desktop.a11y.applications";
    GSettingsSchemaSource *ssrc = g_settings_schema_source_get_default();
    g_autoptr(GSettingsSchema) schema = NULL;

    if (!ssrc) {
        g_warning("No gsettings schemas installed.");
        return;
    }
    schema = g_settings_schema_source_lookup(ssrc, schema_name, TRUE);
    if (schema) {
        g_autoptr(GSettings) settings = g_settings_new (schema_name);
        g_settings_bind (settings, "screen-keyboard-enabled",
                         self, "enabled", G_SETTINGS_BIND_GET);
    } else {
        g_warning("Gsettings schema %s is not installed on the system. "
                  "Enabling by default.", schema_name);
    }
}

ServerContextService *
server_context_service_new (EekboardContextService *self, struct submission *submission, struct squeek_layout_state *layout, struct ui_manager *uiman,  struct squeek_state_manager *state_manager)
{
    ServerContextService *ui = g_object_new (SERVER_TYPE_CONTEXT_SERVICE, NULL);
    ui->submission = submission;
    ui->state = self;
    ui->layout = layout;
    ui->manager = uiman;
    ui->state_manager = state_manager;
    init(ui);
    return ui;
}

// Used from Rust
void server_context_service_set_hint_purpose(ServerContextService *self, uint32_t hint,
                                              uint32_t purpose) {
    eekboard_context_service_set_hint_purpose(self->state, hint, purpose);
}
