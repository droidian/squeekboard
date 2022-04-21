#include "eekboard/eekboard-context-service.h"
#include "wayland.h"
#include "panel.h"


// Called from rust
/// Destroys the widget
void
panel_manager_hide(struct panel_manager *self)
{
    if (self->window) {
        gtk_widget_destroy (GTK_WIDGET (self->window));
    }
    if (self->widget) {
        gtk_widget_destroy (GTK_WIDGET (self->widget));
    }
    self->window = NULL;
    self->widget = NULL;
}

static void
on_destroy (struct panel_manager *self, GtkWidget *widget)
{
    g_assert (widget == GTK_WIDGET(self->window));
    panel_manager_hide(self);
}


/// panel::Manager. Only needed for this callback
struct squeek_panel_manager;

/// Calls back into Rust
void squeek_panel_manager_configured(struct squeek_panel_manager *mgr, uint32_t width, uint32_t height);

static void
on_surface_configure(struct squeek_panel_manager *self, PhoshLayerSurface *surface)
{
    gint width;
    gint height;
    g_return_if_fail (PHOSH_IS_LAYER_SURFACE (surface));

    g_object_get(G_OBJECT(surface),
                 "configured-width", &width,
                 "configured-height", &height,
                 NULL);
    squeek_panel_manager_configured(self, width, height);
}

static void
make_widget (struct panel_manager *self)
{
    if (self->widget) {
        g_error("Widget already present");
    }
    self->widget = eek_gtk_keyboard_new (self->state, self->submission, self->layout);

    gtk_widget_set_has_tooltip (self->widget, TRUE);
    gtk_container_add (GTK_CONTAINER(self->window), self->widget);
    gtk_widget_show_all(self->widget);
}


// Called from rust
/// Creates a new panel widget
void
panel_manager_request_widget (struct panel_manager *self, struct wl_output *output, uint32_t height, struct squeek_panel_manager *mgr)
{
    if (self->window) {
        g_error("Window already present");
    }

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
        "swapped-signal::configured", G_CALLBACK(on_surface_configure), mgr,
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

    make_widget(self);

    gtk_widget_show (GTK_WIDGET(self->window));
}

// Called from rust
/// Updates the size
void
panel_manager_resize (struct panel_manager *self, uint32_t height)
{
    phosh_layer_surface_set_size(self->window, 0, height);
    phosh_layer_surface_set_exclusive_zone(self->window, height);
    phosh_layer_surface_wl_surface_commit(self->window);
}


struct panel_manager panel_manager_new(EekboardContextService *state, struct submission *submission, struct squeek_layout_state *layout)
{
    struct panel_manager mgr = {
        .state = state,
        .submission = submission,
        .layout = layout,
        .window = NULL,
        .widget = NULL,
        .current_output = NULL,
    };
    return mgr;
}
