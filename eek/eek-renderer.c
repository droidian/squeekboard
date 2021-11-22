/*
 * Copyright (C) 2010-2011 Daiki Ueno <ueno@unixuser.org>
 * Copyright (C) 2010-2011 Red Hat, Inc.
 *
 * This library is free software; you can redistribute it and/or
 * modify it under the terms of the GNU Lesser General Public License
 * as published by the Free Software Foundation; either version 2 of
 * the License, or (at your option) any later version.
 *
 * This library is distributed in the hope that it will be useful, but
 * WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
 * Lesser General Public License for more details.
 *
 * You should have received a copy of the GNU Lesser General Public
 * License along with this library; if not, write to the Free Software
 * Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA
 * 02110-1301 USA
 */

#define G_LOG_DOMAIN "squeekboard-eek-renderer"

#include <math.h>
#include <string.h>
#include <gdk-pixbuf/gdk-pixbuf.h>

#include "eek-keyboard.h"
#include "eek-renderer.h"
#include "src/style.h"


/* eek-keyboard-drawing.c */
static void render_button_label (cairo_t *cr, GtkStyleContext *ctx,
                                                const gchar *label, EekBounds bounds);

static void
render_outline (cairo_t     *cr,
                GtkStyleContext *ctx,
                EekBounds bounds)
{
    GtkBorder margin, border;
    gtk_style_context_get_margin(ctx, GTK_STATE_FLAG_NORMAL, &margin);
    gtk_style_context_get_border(ctx, GTK_STATE_FLAG_NORMAL, &border);

    gdouble x = margin.left + border.left;
    gdouble y = margin.top + border.top;
    EekBounds position = {
        .x = x,
        .y = y,
        .width = bounds.width - x - (margin.right + border.right),
        .height = bounds.height - y - (margin.bottom + border.bottom),
    };
    gtk_render_background (ctx, cr,
        position.x, position.y, position.width, position.height);
    gtk_render_frame (ctx, cr,
        position.x, position.y, position.width, position.height);
}

/// Rust interface
void eek_render_button_in_context(uint32_t scale_factor,
                                     cairo_t     *cr,
                                     GtkStyleContext *ctx,
                                     EekBounds bounds,
                                     const char *icon_name,
                                     const gchar *label) {
    /* blank background */
    cairo_set_source_rgba (cr, 0.0, 0.0, 0.0, 0.0);
    cairo_paint (cr);

    render_outline (cr, ctx, bounds);
    cairo_paint (cr);

    /* render icon (if any) */
    if (icon_name) {
        cairo_surface_t *icon_surface =
            eek_renderer_get_icon_surface (icon_name, 16, scale_factor);
        if (icon_surface) {
            gint width = cairo_image_surface_get_width (icon_surface);
            gint height = cairo_image_surface_get_height (icon_surface);

            cairo_save (cr);
            cairo_translate (cr,
                             (bounds.width - (double)width / scale_factor) / 2,
                             (bounds.height - (double)height / scale_factor) / 2);
            cairo_rectangle (cr, 0, 0, width, height);
            cairo_clip (cr);
            /* Draw the shape of the icon using the foreground color */
            GdkRGBA color = {0};
            gtk_style_context_get_color (ctx, GTK_STATE_FLAG_NORMAL, &color);

            cairo_set_source_rgba (cr, color.red,
                                       color.green,
                                       color.blue,
                                       color.alpha);
            cairo_mask_surface (cr, icon_surface, 0.0, 0.0);
            cairo_surface_destroy(icon_surface);
            cairo_fill (cr);
            cairo_restore (cr);
            return;
        }
    }

    if (label) {
        render_button_label (cr, ctx, label, bounds);
    }
}

/// Prepare context for drawing the button.
/// The context MUST be released using the corresponing "put" procedure
/// before drawing the next button.
/// Interface for Rust.
GtkStyleContext *
eek_get_style_context_for_button (EekRenderer *self,
                                  const char *name,
                                  const char *outline_name,
                                  const char *locked_class,
                                  uint64_t     pressed)
{
    GtkStyleContext *ctx = self->button_context;
    /* Set the name of the button on the widget path, using the name obtained
       from the button's symbol. */
    g_autoptr (GtkWidgetPath) path = NULL;
    path = gtk_widget_path_copy (gtk_style_context_get_path (ctx));
    gtk_widget_path_iter_set_name (path, -1, name);

    /* Update the style context with the updated widget path. */
    gtk_style_context_set_path (ctx, path);
    /* Set the state to take into account whether the button is active
       (pressed) or normal. */
    gtk_style_context_set_state(ctx,
        pressed ? GTK_STATE_FLAG_ACTIVE : GTK_STATE_FLAG_NORMAL);
    if (locked_class) {
        gtk_style_context_add_class(ctx, locked_class);
    }
    gtk_style_context_add_class(ctx, outline_name);
    return ctx;
}

/// Interface for Rust.
void eek_put_style_context_for_button(GtkStyleContext *ctx,
                                      const char *outline_name,
                                      const char *locked_class) {
    // Save and restore functions don't work if gtk_render_* was used in between
    gtk_style_context_set_state(ctx, GTK_STATE_FLAG_NORMAL);
    gtk_style_context_remove_class(ctx, outline_name);
    if (locked_class) {
        gtk_style_context_remove_class(ctx, locked_class);
    }
}

static void
render_button_label (cairo_t     *cr,
                     GtkStyleContext *ctx,
                     const gchar *label,
                     EekBounds bounds)
{
    PangoFontDescription *font;
    gtk_style_context_get(ctx,
                          gtk_style_context_get_state(ctx),
                          "font", &font,
                          NULL);
    PangoLayout *layout = pango_cairo_create_layout (cr);
    pango_layout_set_font_description (layout, font);
    pango_font_description_free (font);

    pango_layout_set_text (layout, label, -1);
    PangoLayoutLine *line = pango_layout_get_line_readonly(layout, 0);
    if (line->resolved_dir == PANGO_DIRECTION_RTL) {
        pango_layout_set_alignment (layout, PANGO_ALIGN_RIGHT);
    }
    pango_layout_set_width (layout, PANGO_SCALE * bounds.width);

    PangoRectangle extents = { 0, };
    pango_layout_get_extents (layout, NULL, &extents);

    cairo_save (cr);
    cairo_move_to
        (cr,
         (bounds.width - (double)extents.width / PANGO_SCALE) / 2,
         (bounds.height - (double)extents.height / PANGO_SCALE) / 2);

    GdkRGBA color = {0};
    gtk_style_context_get_color (ctx, GTK_STATE_FLAG_NORMAL, &color);

    cairo_set_source_rgba (cr,
                           color.red,
                           color.green,
                           color.blue,
                           color.alpha);
    pango_cairo_show_layout (cr, layout);
    cairo_restore (cr);
    g_object_unref (layout);
}

// FIXME: Pass just the active modifiers instead of entire submission
void
eek_renderer_render_keyboard (EekRenderer *self,
                              struct render_geometry geometry,
                              struct submission *submission,
                                   cairo_t     *cr,
                              LevelKeyboard *keyboard)
{
    g_return_if_fail (geometry.allocation_width > 0.0);
    g_return_if_fail (geometry.allocation_height > 0.0);

    /* Paint the background covering the entire widget area */
    gtk_render_background (self->view_context,
                           cr,
                           0, 0,
                           geometry.allocation_width, geometry.allocation_height);

    cairo_save(cr);
    cairo_translate (cr, geometry.widget_to_layout.origin_x, geometry.widget_to_layout.origin_y);
    cairo_scale (cr, geometry.widget_to_layout.scale, geometry.widget_to_layout.scale);

    squeek_draw_layout_base_view(keyboard->layout, self, cr);
    squeek_layout_draw_all_changed(keyboard->layout, self, cr, submission);
    cairo_restore (cr);
}

void
eek_renderer_free (EekRenderer        *self)
{
    if (self->pcontext) {
        g_object_unref (self->pcontext);
        self->pcontext = NULL;
    }
    g_object_unref(self->css_provider);
    g_object_unref(self->view_context);
    g_object_unref(self->button_context);
    g_clear_signal_handler (&self->theme_name_id, gtk_settings_get_default());

    // this is where renderer-specific surfaces would be released

    free(self);
}

static GType new_type(char *name) {
    GTypeInfo info = {0};
    info.class_size = sizeof(GtkWidgetClass);
    info.instance_size = sizeof(GtkWidget);

    return g_type_register_static(GTK_TYPE_WIDGET, name, &info,
        G_TYPE_FLAG_ABSTRACT
    );
}

static GType view_type(void) {
    static GType type = 0;
    if (!type) {
        type = new_type("sq_view");
    }
    return type;
}

static GType button_type(void) {
    static GType type = 0;
    if (!type) {
        type = new_type("sq_button");
    }
    return type;
}


static void
on_gtk_theme_name_changed (GtkSettings *settings, gpointer foo, EekRenderer *self)
{
  g_autofree char *name = NULL;

  g_object_get (settings, "gtk-theme-name", &name, NULL);
  g_debug ("GTK theme: %s", name);

  gtk_style_context_remove_provider_for_screen (gdk_screen_get_default (),
                                                GTK_STYLE_PROVIDER (self->css_provider));
  gtk_style_context_remove_provider (self->button_context,
                                     GTK_STYLE_PROVIDER(self->css_provider));
  gtk_style_context_remove_provider (self->view_context,
                                     GTK_STYLE_PROVIDER(self->css_provider));

  g_set_object (&self->css_provider, squeek_load_style());

  gtk_style_context_add_provider_for_screen (gdk_screen_get_default (),
                                             GTK_STYLE_PROVIDER (self->css_provider),
                                             GTK_STYLE_PROVIDER_PRIORITY_APPLICATION);
  gtk_style_context_add_provider (self->button_context,
                                  GTK_STYLE_PROVIDER(self->css_provider),
                                  GTK_STYLE_PROVIDER_PRIORITY_APPLICATION);
  gtk_style_context_add_provider (self->view_context,
                                  GTK_STYLE_PROVIDER(self->css_provider),
                                  GTK_STYLE_PROVIDER_PRIORITY_APPLICATION);
}


static void
renderer_init (EekRenderer *self)
{
    self->pcontext = NULL;
    self->scale_factor = 1;

    GtkSettings *gtk_settings;

    gtk_settings = gtk_settings_get_default ();

    self->theme_name_id = g_signal_connect (gtk_settings, "notify::gtk-theme-name",
                                            G_CALLBACK (on_gtk_theme_name_changed), self);

    self->css_provider = squeek_load_style();
}

EekRenderer *
eek_renderer_new (LevelKeyboard  *keyboard,
                  PangoContext *pcontext)
{
    EekRenderer *renderer = calloc(1, sizeof(EekRenderer));
    renderer_init(renderer);
    renderer->pcontext = pcontext;
    g_object_ref (renderer->pcontext);

    /* Create a style context for the layout */
    GtkWidgetPath *path = gtk_widget_path_new();
    gtk_widget_path_append_type(path, view_type());

    renderer->view_context = gtk_style_context_new();
    gtk_style_context_set_path(renderer->view_context, path);
    gtk_widget_path_unref(path);
    if (squeek_layout_get_kind(keyboard->layout) == ARRANGEMENT_KIND_WIDE) {
        gtk_style_context_add_class(renderer->view_context, "wide");
    }
    gtk_style_context_add_provider (renderer->view_context,
        GTK_STYLE_PROVIDER(renderer->css_provider),
        GTK_STYLE_PROVIDER_PRIORITY_APPLICATION);

    /* Create a style context for the buttons */
    path = gtk_widget_path_new();
    gtk_widget_path_append_type(path, view_type());
    if (squeek_layout_get_kind(keyboard->layout) == ARRANGEMENT_KIND_WIDE) {
        gtk_widget_path_iter_add_class(path, -1, "wide");
    }
    gtk_widget_path_append_type(path, button_type());
    renderer->button_context = gtk_style_context_new ();
    gtk_style_context_set_path(renderer->button_context, path);
    gtk_widget_path_unref(path);
    gtk_style_context_set_parent(renderer->button_context, renderer->view_context);
    gtk_style_context_set_state (renderer->button_context, GTK_STATE_FLAG_NORMAL);
    gtk_style_context_add_provider (renderer->button_context,
        GTK_STYLE_PROVIDER(renderer->css_provider),
        GTK_STYLE_PROVIDER_PRIORITY_APPLICATION);
    return renderer;
}

struct render_geometry
eek_render_geometry_from_allocation_size (struct squeek_layout *layout,
                                  gdouble      width,
                                  gdouble      height)
{
    struct render_geometry ret = {
        .allocation_width = width,
        .allocation_height = height,
        .widget_to_layout = squeek_layout_calculate_transformation(
            layout, width, height),
    };
    return ret;
}

void
eek_renderer_set_scale_factor (EekRenderer *renderer, gint scale)
{
    renderer->scale_factor = scale;
}

/// Rust interface.
uint32_t eek_renderer_get_scale_factor(EekRenderer *renderer) {
    return renderer->scale_factor;
}

cairo_surface_t *
eek_renderer_get_icon_surface (const gchar *icon_name,
                               gint size,
                               gint scale)
{
    GError *error = NULL;
    cairo_surface_t *surface = gtk_icon_theme_load_surface (gtk_icon_theme_get_default (),
                                                            icon_name,
                                                            size,
                                                            scale,
                                                            NULL,
                                                            0,
                                                            &error);

    if (surface == NULL) {
        g_warning ("can't get icon surface for %s: %s",
                   icon_name,
                   error->message);
        g_error_free (error);
        return NULL;
    }
    return surface;
}
